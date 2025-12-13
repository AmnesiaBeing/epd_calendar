// src/kernel/data/scheduler.rs
//! 数据源调度器模块
//! 实现单静态任务的数据源调度
use alloc::collections::BTreeMap;
use embassy_time::{Duration, Instant, Ticker};
use static_cell::StaticCell;

use crate::common::error::{AppError, Result};
use crate::common::{GlobalMutex, GlobalRwLock, GlobalRwLockReadGuard, GlobalRwLockWriteGuard};
use crate::kernel::data::types::{CacheKeyValueMap, HeaplessVec};
use crate::kernel::data::{DataSource, DynamicValue};
use crate::kernel::system::api::DefaultSystemApi;
use crate::tasks::{DISPLAY_EVENTS, DisplayEvent};

// ========== 常量定义（提升可维护性） ==========
/// 默认最小刷新间隔（秒）
const DEFAULT_MIN_INTERVAL_SECS: u64 = 60;
/// 数据源刷新重试次数
const REFRESH_RETRY_COUNT: u8 = 1;
/// 数据源列表最大容量（匹配heapless::Vec的固定容量）
const MAX_SOURCES: usize = 8;

// 为常用的固定长度heapless类型创建别名
pub type DataSourceList = HeaplessVec<SourceMeta, MAX_SOURCES>;
/// 缓存key拼接分隔符
const CACHE_KEY_SEP: &str = ".";

// ========== 静态资源初始化（优化安全性） ==========
/// 创建数据源调度器（StaticCell确保仅初始化一次）
static DATA_SOURCE_SCHEDULER: StaticCell<GlobalMutex<DataSourceRegistry>> = StaticCell::new();
/// 下一个可用的数据源ID（改为u32，避免类型混淆）
static NEXT_SOURCE_ID: GlobalMutex<u32> = GlobalMutex::new(0);

// ========== 数据源元数据（优化字段命名/注释） ==========
/// 数据源元数据
struct SourceMeta {
    /// 数据源唯一ID（数字标识）
    pub id: u32,
    /// 数据源实例（全局互斥锁保护）
    pub instance: &'static GlobalMutex<dyn DataSource>,
    /// 刷新间隔（毫秒）
    pub refresh_interval: Duration,
    /// 上次刷新时间戳
    pub last_refreshed: Instant,
    /// 数据源名称（预缓存，避免重复调用name()）
    pub source_name: &'static str,
}

// ========== 数据源调度器核心逻辑（核心优化） ==========
/// 数据源调度器
/// 管理所有数据源的定时刷新，适配embassy静态任务特性
pub struct DataSourceRegistry {
    /// 数据源元数据列表（固定容量，避免动态分配）
    sources: DataSourceList,
    /// 最小刷新间隔（所有数据源的最小间隔）
    min_refresh_interval: Duration,
    /// 任意数据源最后更新时间
    last_any_updated: Instant,
    /// 全局数据缓存（读写锁保护，key=数据源名称.字段名）
    cache: GlobalRwLock<CacheKeyValueMap>,
}

impl Default for DataSourceRegistry {
    fn default() -> Self {
        Self {
            sources: HeaplessVec::new(),
            min_refresh_interval: Duration::from_secs(DEFAULT_MIN_INTERVAL_SECS),
            last_any_updated: Instant::MIN,
            // 初始化空BTreeMap，避免默认值分配
            cache: GlobalRwLock::new(BTreeMap::new()),
        }
    }
}

impl DataSourceRegistry {
    // ========== 初始化方法（优化安全性） ==========
    /// 创建新的数据源调度器（确保仅初始化一次）
    pub fn init() -> &'static GlobalMutex<Self> {
        DATA_SOURCE_SCHEDULER.init(GlobalMutex::new(Self::default()))
    }

    // ========== 数据源注册（优化ID管理/缓存预分配） ==========
    /// 静态注册数据源
    pub async fn register_source(
        &mut self,
        instance: &'static GlobalMutex<dyn DataSource>,
    ) -> Result<()> {
        // 1. 安全获取下一个数据源ID（避免重复）
        let mut id_guard = NEXT_SOURCE_ID.lock().await;
        let source_id = *id_guard;
        *id_guard += 1;
        drop(id_guard); // 尽早释放ID锁

        // 2. 获取数据源元信息（预缓存名称，避免重复调用）
        let source_guard = instance.lock().await;
        let source_name = source_guard.name();
        let refresh_interval = source_guard.refresh_interval();
        drop(source_guard); // 尽早释放数据源锁

        // 3. 检查数据源列表容量
        if self.sources.len() >= MAX_SOURCES {
            return Err(AppError::DataSourceRegistryFull);
        }

        // 4. 添加数据源元数据
        self.sources
            .push(SourceMeta {
                id: source_id,
                instance,
                refresh_interval,
                last_refreshed: Instant::MIN, // 初始化为最小时间，首次必刷新
                source_name,
            })
            .map_err(|_| AppError::DataSourceRegistryFull)?;

        // 5. 重新计算最小刷新间隔（提取为辅助方法）
        self.update_min_refresh_interval();

        log::info!(
            "Registered {:?} (new min interval: {}ms)",
            self.sources.last().unwrap().source_name,
            self.min_refresh_interval.as_millis()
        );

        Ok(())
    }

    // ========== 辅助方法：更新最小刷新间隔（提取重复逻辑） ==========
    /// 更新最小刷新间隔（遍历所有数据源取最小值）
    fn update_min_refresh_interval(&mut self) {
        self.min_refresh_interval = self
            .sources
            .iter()
            .map(|meta| meta.refresh_interval)
            .min()
            .unwrap_or(Duration::from_secs(DEFAULT_MIN_INTERVAL_SECS));
    }

    // ========== 缓存接口（供DataSource直写） ==========
    /// 获取缓存写守卫（数据源刷新时调用）
    pub async fn get_cache_write_guard(&self) -> GlobalRwLockWriteGuard<CacheKeyValueMap> {
        self.cache.write().await
    }

    /// 获取缓存读守卫（渲染引擎同步读取）
    pub async fn get_cache_read_guard(&self) -> GlobalRwLockReadGuard<'_, CacheKeyValueMap> {
        self.cache.read().await
    }

    // ========== 缓存读取（优化性能/错误信息） ==========
    /// 异步从缓存获取数据（带详细错误信息）
    pub async fn get_value_by_path_async(&self, path: &str) -> Result<DynamicValue> {
        let cache_guard = self.cache.read().await;
        cache_guard.get(path).cloned().ok_or_else(|| {
            log::error!(
                "Path '{}' not found in cache (cache size: {})",
                path,
                cache_guard.len()
            );
            AppError::CacheMiss
        })
    }

    /// 同步从缓存获取数据（需提前获取缓存守卫）
    pub fn get_value_by_path_sync(
        &self,
        cache: &CacheKeyValueMap,
        path: &str,
    ) -> Result<DynamicValue> {
        cache.get(path).cloned().ok_or_else(|| {
            log::error!(
                "Path '{}' not found in cache (cache size: {})",
                path,
                cache.len()
            );
            AppError::CacheMiss
        })
    }

    // ========== 公共方法（优化接口） ==========
    /// 获取所有数据源的最小刷新间隔
    pub fn get_min_refresh_interval(&self) -> Duration {
        self.min_refresh_interval
    }

    /// 获取任意数据源最后更新时间
    pub fn get_last_any_updated(&self) -> Instant {
        self.last_any_updated
    }

    /// 更新全局最后更新时间（数据源刷新成功后调用）
    pub fn update_last_any_updated(&mut self, time: Instant) {
        self.last_any_updated = time;
    }
}

// ========== 调度任务（修复核心借用问题） ==========
/// 单静态任务：统一轮询所有数据源
/// 按所有数据源的最小刷新间隔定时执行，遍历检查各数据源是否达到刷新时间
#[embassy_executor::task]
pub async fn generic_scheduler_task(
    scheduler: &'static GlobalMutex<DataSourceRegistry>,
    system_api: &'static GlobalMutex<DefaultSystemApi>,
) {
    log::info!("Starting DataSource scheduler task");
    let mut current_min_interval = Duration::MAX;
    let mut ticker: Option<Ticker> = None;

    loop {
        // 1. 更新ticker间隔
        let scheduler_guard = scheduler.lock().await;
        let new_min_interval = scheduler_guard.get_min_refresh_interval();
        drop(scheduler_guard);

        if new_min_interval != current_min_interval {
            current_min_interval = new_min_interval;
            ticker = Some(Ticker::every(current_min_interval));
            log::info!(
                "Scheduler ticker updated to {}ms",
                current_min_interval.as_millis()
            );
        }

        // 2. 等待ticker触发
        let Some(t) = ticker.as_mut() else {
            log::error!("Ticker not initialized, use default interval");
            embassy_time::Timer::after(Duration::from_secs(DEFAULT_MIN_INTERVAL_SECS)).await;
            continue;
        };
        t.next().await;

        log::debug!("Scheduler tick - checking {} data sources", MAX_SOURCES);

        // 3. 收集待刷新的数据源（仅只读遍历，无可变借用）
        let mut pending_refresh = HeaplessVec::<_, MAX_SOURCES>::new();
        {
            let scheduler_guard = scheduler.lock().await;
            let now = Instant::now();
            for meta in scheduler_guard.sources.iter() {
                if now - meta.last_refreshed >= meta.refresh_interval {
                    let _ = pending_refresh.push((meta.id, meta.instance, meta.source_name));
                    log::debug!(
                        "[{}] ready for refresh (idle: {}ms)",
                        meta.id,
                        (now - meta.last_refreshed).as_millis()
                    );
                }
            }
        }

        // 4. 逐个刷新数据源（核心：数据源直写缓存，无借用冲突）
        let mut need_refresh_display = false;
        let now = Instant::now();
        for (source_id, source_instance, _) in pending_refresh {
            // 步骤1：获取缓存写守卫（供数据源直写）
            let mut scheduler_guard = scheduler.lock().await;
            let mut cache_guard = scheduler_guard.get_cache_write_guard().await;

            // 步骤2：刷新数据源并直写缓存
            let mut source = source_instance.lock().await;
            let refresh_result = source
                .refresh_with_cache(system_api, &mut cache_guard)
                .await;
            drop(source); // 释放数据源锁
            drop(cache_guard); // 释放缓存写锁

            // 步骤3：更新状态（仅单次可变借用，无冲突）
            if refresh_result.is_ok() {
                // 找到对应数据源并更新last_refreshed
                if let Some(meta) = scheduler_guard
                    .sources
                    .iter_mut()
                    .find(|m| m.id == source_id)
                {
                    meta.last_refreshed = now;
                    scheduler_guard.update_last_any_updated(now);
                    need_refresh_display = true;
                    log::debug!("[{}] refreshed and cache updated", source_id);
                }
            } else {
                log::warn!(
                    "[{}] refresh failed: {:?}",
                    source_id,
                    refresh_result.unwrap_err()
                );
            }
            drop(scheduler_guard); // 立即释放锁
        }

        // 5. 发送刷新事件（无Result判断）
        if need_refresh_display {
            DISPLAY_EVENTS.send(DisplayEvent::FullRefresh).await;
            log::debug!("Sent FullRefresh event to display task (success)");
        }
    }
}
