// src/kernel/data/scheduler.rs
//! 数据源调度器模块
//!
//! 核心功能：
//! 1. 统一管理所有数据源的注册、定时刷新和数据缓存
//! 2. 确保所有数据源在渲染前完成首次保底数据加载
//! 3. 提供加载状态查询接口，适配显示层的加载中/完成状态切换
//!
//! 设计原则：
//! - 静态化：基于StaticCell实现单例，避免动态分配
//! - 低耦合：数据源仅负责数据生产，调度器负责生命周期和缓存管理
//! - 线程安全：通过GlobalMutex/GlobalRwLock保护所有共享状态
//! - 保底机制：注册时立即触发首次刷新（带超时控制），确保生成保底数据
//! - 超时保护：每个数据源加载设置超时，避免长时间阻塞或死锁

use alloc::collections::BTreeMap;
use embassy_time::{Duration, Instant, Ticker, Timer, WithTimeout};
use static_cell::StaticCell;

use crate::common::error::{AppError, Result};
use crate::common::{GlobalMutex, GlobalRwLock, GlobalRwLockReadGuard, GlobalRwLockWriteGuard};
use crate::kernel::data::types::{CacheKeyValueMap, HeaplessVec};
use crate::kernel::data::{DataSource, DynamicValue};
use crate::kernel::system::api::DefaultSystemApi;
use crate::tasks::{DISPLAY_EVENTS, DisplayEvent};

// ========== 常量定义（提升可维护性与可读性） ==========
/// 默认最小刷新间隔（秒）- 无数据源时的兜底值
const DEFAULT_MIN_INTERVAL_SECS: u64 = 60;
/// 数据源定时刷新重试次数
const REFRESH_RETRY_COUNT: u8 = 1;
/// 数据源列表最大容量（匹配heapless::Vec的固定容量）
const MAX_SOURCES: usize = 8;
/// 数据源首次加载超时时间（毫秒）- 避免长时间阻塞
const INIT_LOAD_TIMEOUT_MS: u64 = 5000; // 5秒超时，可根据业务调整
/// 缓存key拼接分隔符（数据源名称.字段名）
const CACHE_KEY_SEP: &str = ".";

// 类型别名 - 简化常用固定长度容器类型
type DataSourceList = HeaplessVec<SourceMeta, MAX_SOURCES>;

// ========== 静态资源初始化（线程安全的单例模式） ==========
/// 数据源调度器单例（StaticCell确保仅初始化一次）
static DATA_SOURCE_SCHEDULER: StaticCell<GlobalMutex<DataSourceRegistry>> = StaticCell::new();
/// 下一个可用的数据源ID（原子性分配，避免重复）
static NEXT_SOURCE_ID: GlobalMutex<u32> = GlobalMutex::new(0);

// ========== 数据源元数据结构体（存储调度所需核心信息） ==========
/// 数据源元数据
/// 存储调度器管理数据源所需的所有辅助信息，避免重复查询数据源实例
struct SourceMeta {
    /// 数据源唯一数字标识（全局递增）
    pub id: u32,
    /// 数据源实例（全局互斥锁保护，确保线程安全访问）
    pub instance: &'static GlobalMutex<dyn DataSource>,
    /// 数据源自定义刷新间隔（毫秒）
    pub refresh_interval: Duration,
    /// 上次成功刷新时间戳（用于判断是否需要触发新刷新）
    pub last_refreshed: Instant,
    /// 数据源名称（预缓存，避免重复调用name()方法）
    pub source_name: &'static str,
}

// ========== 数据源调度器核心结构体 ==========
/// 数据源调度器
/// 核心职责：
/// 1. 注册并管理所有数据源的生命周期
/// 2. 按最小间隔轮询，触发到期数据源的刷新
/// 3. 维护全局数据缓存，提供读写接口
/// 4. 确保所有数据源完成首次保底数据加载（带超时保护）
pub struct DataSourceRegistry {
    /// 已注册数据源元数据列表（固定容量，无动态分配）
    sources: DataSourceList,
    /// 所有数据源的最小刷新间隔（调度器轮询周期）
    min_refresh_interval: Duration,
    /// 任意数据源最后一次成功更新的时间戳
    last_any_updated: Instant,
    /// 全局数据缓存（读写锁保护，支持多读单写）
    cache: GlobalRwLock<CacheKeyValueMap>,
    /// 数据源初始化状态映射（记录每个数据源是否完成首次保底加载）
    initialized_sources: BTreeMap<u32, bool>,
    /// 所有数据源初始化完成标志（快速查询，避免遍历）
    all_initialized: bool,
}

impl Default for DataSourceRegistry {
    /// 默认初始化
    /// 所有状态置为初始值，缓存初始化为空BTreeMap
    fn default() -> Self {
        Self {
            sources: HeaplessVec::new(),
            min_refresh_interval: Duration::from_secs(DEFAULT_MIN_INTERVAL_SECS),
            last_any_updated: Instant::MIN,
            cache: GlobalRwLock::new(BTreeMap::new()),
            initialized_sources: BTreeMap::new(),
            all_initialized: false,
        }
    }
}

impl DataSourceRegistry {
    // ========== 单例初始化方法 ==========
    /// 初始化数据源调度器单例
    /// 确保全局仅存在一个调度器实例，返回全局可访问的互斥锁引用
    ///
    /// # 返回值
    /// &'static GlobalMutex<Self> - 调度器单例的全局互斥锁引用
    pub fn init() -> &'static GlobalMutex<Self> {
        DATA_SOURCE_SCHEDULER.init(GlobalMutex::new(Self::default()))
    }

    // ========== 数据源注册核心方法 ==========
    /// 静态注册数据源并完成首次保底数据加载
    /// 注册流程：
    /// 1. 分配唯一ID
    /// 2. 获取数据源元信息并缓存
    /// 3. 检查容量并添加元数据
    /// 4. 立即触发首次刷新（带超时控制），确保生成保底数据
    /// 5. 更新最小刷新间隔
    ///
    /// # 参数
    /// - instance: 数据源实例的全局互斥锁引用
    /// - system_api: 系统API实例引用（用于数据源初始化）
    ///
    /// # 返回值
    /// Result<()> - 成功返回Ok(())，失败返回AppError（如容量不足、初始化超时/失败）
    pub async fn register_source(
        &mut self,
        instance: &'static GlobalMutex<dyn DataSource>,
        system_api: &'static GlobalMutex<DefaultSystemApi>,
    ) -> Result<()> {
        // 1. 安全分配唯一数据源ID（尽早释放锁）
        let source_id = {
            let mut id_guard = NEXT_SOURCE_ID.lock().await;
            let id = *id_guard;
            *id_guard += 1;
            id
        };

        // 2. 获取并缓存数据源元信息（减少锁持有时间）
        let (source_name, refresh_interval) = {
            let source_guard = instance.lock().await;
            let name = source_guard.name();
            let interval = source_guard.refresh_interval();
            (name, interval)
        };

        // 3. 容量检查（避免超出固定容量限制）
        if self.sources.len() >= MAX_SOURCES {
            log::error!(
                "Failed to register data source: registry is full (max: {})",
                MAX_SOURCES
            );
            return Err(AppError::DataSourceRegistryFull);
        }

        // 4. 添加数据源元数据到列表
        let meta = SourceMeta {
            id: source_id,
            instance,
            refresh_interval,
            last_refreshed: Instant::MIN, // 初始化为最小时间，确保首次必刷新
            source_name,
        };
        self.sources.push(meta).map_err(|_| {
            log::error!("Failed to register data source: failed to add meta to list");
            AppError::DataSourceRegistryFull
        })?;

        // 5. 初始化数据源状态（标记为未初始化）
        self.initialized_sources.insert(source_id, false);
        self.all_initialized = false; // 新增数据源后重置全局初始化状态

        // 6. 立即触发首次刷新（带超时控制），确保生成保底数据
        log::info!(
            "Start initializing fallback data for data source [{}:{}]",
            source_id,
            source_name
        );
        let init_result = self.init_source_with_timeout(source_id, system_api).await;
        if init_result.is_err() {
            log::error!(
                "Failed to initialize fallback data for data source [{}:{}] (timeout: {}ms)",
                source_id,
                source_name,
                INIT_LOAD_TIMEOUT_MS
            );
            return init_result;
        }

        // 7. 更新最小刷新间隔（确保调度器轮询周期准确）
        self.update_min_refresh_interval();

        // 8. 检查并更新全局初始化状态
        self.check_all_initialized();

        log::info!(
            "Data source [{}:{}] registered successfully (count: {}, min interval: {}ms)",
            source_id,
            source_name,
            self.sources.len(),
            self.min_refresh_interval.as_millis()
        );

        Ok(())
    }

    // ========== 保底数据初始化辅助方法 ==========
    /// 带超时控制的数据源初始化（生成保底数据）
    /// 首次注册时调用，确保数据源至少有基础数据写入缓存，避免长时间阻塞
    ///
    /// # 参数
    /// - source_id: 数据源唯一ID
    /// - system_api: 系统API实例引用
    ///
    /// # 返回值
    /// Result<()> - 成功返回Ok(())，超时/失败返回Err
    async fn init_source_with_timeout(
        &mut self,
        source_id: u32,
        system_api: &'static GlobalMutex<DefaultSystemApi>,
    ) -> Result<()> {
        // 第一步：先读取数据源的必要信息，避免持有不可变引用
        let (source_instance, source_name) = {
            let meta = self
                .sources
                .iter()
                .find(|m| m.id == source_id)
                .ok_or(AppError::DataSourceNotFound)?;
            (meta.instance, meta.source_name)
        };

        log::debug!(
            "Initializing data source [{}:{}] with timeout: {}ms",
            source_id,
            source_name,
            INIT_LOAD_TIMEOUT_MS
        );

        // 封装加载逻辑为异步闭包，用于超时控制
        let load_task = async {
            // 获取缓存写锁和数据源实例锁（尽可能短的持有时间）
            let mut cache_guard = self.cache.write().await;
            let mut source_guard = source_instance.lock().await;

            // 触发数据源刷新（写入保底数据）
            let result = source_guard
                .refresh_with_cache(system_api, &mut cache_guard)
                .await;

            // 尽早释放锁，提高并发性
            drop(source_guard);
            drop(cache_guard);

            result
        };

        match load_task
            .with_timeout(Duration::from_millis(INIT_LOAD_TIMEOUT_MS))
            .await
        {
            Ok(Ok(_)) => {
                // 初始化成功：更新状态
                self.mark_source_initialized(source_id);
                self.last_any_updated = Instant::now();

                // 更新数据源的last_refreshed
                if let Some(meta) = self.sources.iter_mut().find(|m| m.id == source_id) {
                    meta.last_refreshed = Instant::now();
                }

                log::info!(
                    "Fallback data initialized successfully for data source [{}:{}]",
                    source_id,
                    source_name
                );
                Ok(())
            }
            Ok(Err(e)) => {
                // 加载任务内部失败
                log::error!(
                    "Failed to initialize data source [{}:{}]: {:?}",
                    source_id,
                    source_name,
                    e
                );
                Err(AppError::DataSourceInitFailed)
            }
            Err(_) => {
                // 加载超时
                log::error!(
                    "Data source [{}:{}] initialization timed out ({}ms)",
                    source_id,
                    source_name,
                    INIT_LOAD_TIMEOUT_MS
                );
                Err(AppError::DataSourceInitTimeout)
            }
        }
    }

    /// 标记数据源为已初始化
    ///
    /// # 参数
    /// - source_id: 数据源唯一ID
    fn mark_source_initialized(&mut self, source_id: u32) {
        self.initialized_sources.insert(source_id, true);
    }

    /// 检查所有已注册数据源是否完成初始化
    /// 更新all_initialized标志，供外部快速查询
    fn check_all_initialized(&mut self) {
        if self.sources.is_empty() {
            self.all_initialized = false;
            return;
        }

        // 检查所有数据源是否都已初始化
        let all_init = self
            .sources
            .iter()
            .all(|meta| self.initialized_sources.get(&meta.id) == Some(&true));

        self.all_initialized = all_init;
        if all_init {
            log::info!(
                "All {} data sources have completed fallback data initialization",
                self.sources.len()
            );
        }
    }

    // ========== 加载状态查询接口（供显示线程使用） ==========
    /// 查询是否所有数据源都完成保底数据初始化
    /// 显示线程可通过此接口判断是否切换出加载中页面
    ///
    /// # 返回值
    /// bool - true: 所有数据源已初始化；false: 仍有未初始化的数据源
    pub async fn is_all_initialized(&self) -> bool {
        self.all_initialized
    }

    /// 等待所有数据源完成初始化（异步阻塞）
    /// 显示线程可调用此方法等待加载完成，超时返回false
    ///
    /// # 参数
    /// - timeout: 最大等待时长
    ///
    /// # 返回值
    /// bool - true: 所有数据源初始化完成；false: 超时
    pub async fn wait_for_all_initialized(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            if self.all_initialized {
                return true;
            }
            if Instant::now() - start >= timeout {
                log::warn!(
                    "Wait for data sources initialization timed out ({}ms)",
                    timeout.as_millis()
                );
                return false;
            }
            // 短轮询（减少CPU占用）
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    // ========== 调度器核心辅助方法 ==========
    /// 更新最小刷新间隔
    /// 遍历所有数据源，取最小的刷新间隔作为调度器轮询周期
    fn update_min_refresh_interval(&mut self) {
        self.min_refresh_interval = self
            .sources
            .iter()
            .map(|meta| meta.refresh_interval)
            .min()
            .unwrap_or(Duration::from_secs(DEFAULT_MIN_INTERVAL_SECS));
    }

    // ========== 缓存操作接口 ==========
    /// 获取缓存写守卫（供数据源刷新时写入数据）
    ///
    /// # 返回值
    /// GlobalRwLockWriteGuard - 缓存的写锁守卫
    pub async fn get_cache_write_guard(&self) -> GlobalRwLockWriteGuard<'_, CacheKeyValueMap> {
        self.cache.write().await
    }

    /// 获取缓存读守卫（供渲染引擎读取数据）
    ///
    /// # 返回值
    /// GlobalRwLockReadGuard - 缓存的读锁守卫
    pub async fn get_cache_read_guard(&self) -> GlobalRwLockReadGuard<'_, CacheKeyValueMap> {
        self.cache.read().await
    }

    /// 异步从缓存获取指定路径的数据
    /// 路径格式：数据源名称.字段名
    ///
    /// # 参数
    /// - path: 缓存数据路径
    ///
    /// # 返回值
    /// Result<DynamicValue> - 成功返回数据，失败返回CacheMiss
    pub async fn get_value_by_path_async(&self, path: &str) -> Result<DynamicValue> {
        let cache_guard = self.cache.read().await;
        cache_guard.get(path).cloned().ok_or_else(|| {
            log::error!(
                "Cache miss for path '{}' (cache size: {})",
                path,
                cache_guard.len()
            );
            AppError::CacheMiss
        })
    }

    /// 同步从缓存获取指定路径的数据（需提前获取缓存守卫）
    /// 适用于已持有缓存锁的场景，避免重复加锁
    ///
    /// # 参数
    /// - cache: 缓存引用（已获取读锁）
    /// - path: 缓存数据路径
    ///
    /// # 返回值
    /// Result<DynamicValue> - 成功返回数据，失败返回CacheMiss
    pub fn get_value_by_path_sync(
        &self,
        cache: &CacheKeyValueMap,
        path: &str,
    ) -> Result<DynamicValue> {
        cache.get(path).cloned().ok_or_else(|| {
            log::error!(
                "Cache miss for path '{}' (cache size: {})",
                path,
                cache.len()
            );
            AppError::CacheMiss
        })
    }

    // ========== 只读属性访问器 ==========
    /// 获取所有数据源的最小刷新间隔
    ///
    /// # 返回值
    /// Duration - 最小刷新间隔
    pub fn get_min_refresh_interval(&self) -> Duration {
        self.min_refresh_interval
    }

    /// 获取任意数据源最后一次更新的时间戳
    ///
    /// # 返回值
    /// Instant - 最后更新时间
    pub fn get_last_any_updated(&self) -> Instant {
        self.last_any_updated
    }

    /// 更新全局最后更新时间戳
    /// 数据源刷新成功后调用
    ///
    /// # 参数
    /// - time: 新的更新时间戳
    pub fn update_last_any_updated(&mut self, time: Instant) {
        self.last_any_updated = time;
    }
}

// ========== 调度器核心任务 ==========
/// 数据源调度器核心任务
/// 功能：
/// 1. 按最小刷新间隔轮询所有数据源
/// 2. 触发到期数据源的定时刷新
/// 3. 刷新完成后发送显示刷新事件
///
/// # 参数
/// - scheduler: 调度器单例的全局互斥锁引用
/// - system_api: 系统API实例的全局互斥锁引用
#[embassy_executor::task]
pub async fn generic_scheduler_task(
    scheduler: &'static GlobalMutex<DataSourceRegistry>,
    system_api: &'static GlobalMutex<DefaultSystemApi>,
) {
    log::info!("Data source scheduler task started");
    let mut current_min_interval = Duration::MAX;
    let mut ticker: Option<Ticker> = None;

    loop {
        // 1. 更新轮询ticker（当最小刷新间隔变化时）
        let new_min_interval = {
            let scheduler_guard = scheduler.lock().await;
            scheduler_guard.get_min_refresh_interval()
        };

        if new_min_interval != current_min_interval {
            current_min_interval = new_min_interval;
            ticker = Some(Ticker::every(current_min_interval));
            log::info!(
                "Scheduler ticker updated to {}ms",
                current_min_interval.as_millis()
            );
        }

        // 2. 等待轮询触发（兜底处理ticker未初始化的情况）
        let Some(t) = ticker.as_mut() else {
            log::warn!(
                "Scheduler ticker not initialized, using default interval {}s",
                DEFAULT_MIN_INTERVAL_SECS
            );
            Timer::after(Duration::from_secs(DEFAULT_MIN_INTERVAL_SECS)).await;
            continue;
        };
        t.next().await;

        log::debug!("Scheduler tick - checking data sources refresh status");

        // 3. 收集需要刷新的数据源（只读遍历，减少锁持有时间）
        let mut pending_refresh = HeaplessVec::<_, MAX_SOURCES>::new();
        {
            let scheduler_guard = scheduler.lock().await;
            let now = Instant::now();
            for meta in scheduler_guard.sources.iter() {
                // 判断是否达到刷新间隔
                if now - meta.last_refreshed >= meta.refresh_interval {
                    let _ = pending_refresh.push((meta.id, meta.instance, meta.source_name));
                    log::debug!(
                        "Data source [{}:{}] ready for refresh (idle: {}ms)",
                        meta.id,
                        meta.source_name,
                        (now - meta.last_refreshed).as_millis()
                    );
                }
            }
        }

        // 4. 逐个刷新数据源（核心逻辑）
        let mut need_refresh_display = false;
        let now = Instant::now();
        for (source_id, source_instance, source_name) in pending_refresh {
            // 步骤1：获取缓存写锁（供数据源写入新数据）
            let mut scheduler_guard = scheduler.lock().await;
            let mut cache_guard = scheduler_guard.get_cache_write_guard().await;

            // 步骤2：触发数据源刷新
            let mut source = source_instance.lock().await;
            let refresh_result = source
                .refresh_with_cache(system_api, &mut cache_guard)
                .await;
            drop(source); // 尽早释放数据源锁
            drop(cache_guard); // 尽早释放缓存锁

            // 步骤3：更新刷新状态
            if refresh_result.is_ok() {
                // 更新数据源最后刷新时间
                if let Some(meta) = scheduler_guard
                    .sources
                    .iter_mut()
                    .find(|m| m.id == source_id)
                {
                    meta.last_refreshed = now;
                    scheduler_guard.update_last_any_updated(now);
                    need_refresh_display = true;
                    log::debug!(
                        "Data source [{}:{}] refreshed successfully",
                        source_id,
                        source_name
                    );
                }
            } else {
                log::warn!(
                    "Failed to refresh data source [{}:{}]: {:?}",
                    source_id,
                    source_name,
                    refresh_result.unwrap_err()
                );
            }
            drop(scheduler_guard); // 释放调度器锁
        }

        // 5. 发送显示刷新事件（仅当有数据源刷新成功时）
        if need_refresh_display {
            DISPLAY_EVENTS.send(DisplayEvent::FullRefresh).await;
            log::debug!("FullRefresh event sent to display task successfully");
        }
    }
}
