// src/kernel/data/scheduler.rs
//! 数据源调度器模块
//! 实现单静态任务的数据源调度

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::data::{DataSource, DataSourceId, DynamicValue};
use crate::kernel::system::api::DefaultSystemApi;
use crate::tasks::{DISPLAY_EVENTS, DisplayEvent};

use embassy_time::{Duration, Instant, Ticker};
use heapless::Vec;
use static_cell::StaticCell;

/// 创建数据源调度器
static DATA_SOURCE_SCHEDULER: StaticCell<GlobalMutex<DataSourceRegistry>> = StaticCell::new();

/// 下一个可用的数据源ID
static NEXT_ID: GlobalMutex<DataSourceId> = GlobalMutex::new(0);

/// 数据源元数据
struct SourceMeta {
    /// 数据源ID
    pub id: DataSourceId,
    /// 数据源实例
    pub instance: &'static GlobalMutex<dyn DataSource>,
    /// 刷新间隔
    pub interval_tick: Duration,
    /// 上次刷新时间
    pub last_refresh_tick: Instant,
}

/// 数据源调度器
/// 管理所有数据源的定时刷新，适配embassy静态任务特性
pub struct DataSourceRegistry {
    /// 数据源元数据列表
    sources: Vec<SourceMeta, 8>,
    /// 最小刷新间隔
    min_interval_tick: Duration,
    /// 任何数据源的最后更新时间
    last_any_updated: Instant,
}

impl Default for DataSourceRegistry {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            min_interval_tick: Duration::from_secs(60), // 默认最小间隔60秒
            last_any_updated: Instant::MIN,             // 任何数据源的最后更新时间
        }
    }
}

impl DataSourceRegistry {
    /// 创建新的数据源调度器
    pub fn new() -> &'static GlobalMutex<Self> {
        DATA_SOURCE_SCHEDULER.init(GlobalMutex::new(DataSourceRegistry::default()))
    }

    /// 静态注册数据源
    pub async fn register_source(
        &mut self,
        instance: &'static GlobalMutex<dyn DataSource>,
    ) -> Result<()> {
        // 获取下一个可用ID
        let id = *NEXT_ID.lock().await;
        *NEXT_ID.lock().await += 1;

        let interval = instance.lock().await.refresh_interval();

        // 添加数据源元数据
        self.sources
            .push(SourceMeta {
                id,
                instance,
                interval_tick: interval,
                last_refresh_tick: Instant::MIN,
            })
            .map_err(|_| AppError::DataSourceRegistryFull)?;

        // 重新计算最小刷新间隔
        self.min_interval_tick = self.get_min_interval();
        log::info!(
            "Registered DataSource {:?}, interval: {}s, new min interval: {}s",
            id,
            interval / 1000,
            self.min_interval_tick / 1000
        );

        Ok(())
    }

    /// 计算所有数据源的最小刷新间隔
    pub fn get_min_interval(&self) -> Duration {
        if self.sources.is_empty() {
            return Duration::from_secs(60); // 默认60秒
        }

        // 取所有数据源间隔的最小值
        self.sources
            .iter()
            .map(|s| s.interval_tick)
            .min()
            .unwrap_or(Duration::from_secs(60))
    }

    /// 刷新所有数据源
    pub async fn refresh_all(
        &mut self,
        system_api: &'static GlobalMutex<DefaultSystemApi>,
    ) -> Result<()> {
        let now = Instant::now();

        for source_meta in self.sources.iter_mut() {
            // 执行刷新
            let mut source = source_meta.instance.lock().await;
            if source.refresh(system_api).await.is_ok() {
                source_meta.last_refresh_tick = now;
                // 更新最后更新时间
                self.last_any_updated = now;
                log::debug!("[{}] Refreshed in refresh_all", source_meta.id);
            }
        }

        Ok(())
    }

    /// 通过字符串路径获取数据（异步版本）
    /// 路径格式：数据源名称.字段名称，例如："config.wifi_ssid"、"datetime.date.year"
    pub async fn get_value_by_path(&self, path: &str) -> Result<DynamicValue> {
        // 解析路径，分离数据源名称和字段名称
        let parts: Vec<&str, 2> = path.split('.').collect();
        if parts.len() < 2 {
            return Err(AppError::InvalidPathFormat);
        }

        let source_name = parts[0];
        let field_name = parts[1..].join(".");

        // 查找对应的数据源
        for source_meta in self.sources.iter() {
            let source = source_meta.instance.lock().await;
            if source.name() == source_name {
                // 从数据源中获取字段值
                let value = source.get_field_value(&field_name)?;
                return Ok(value);
            }
        }

        // 未找到数据源
        Err(AppError::DataSourceNotFound)
    }

    /// 获取任何数据源的最后更新时间
    pub fn get_last_any_updated(&self) -> Instant {
        self.last_any_updated
    }
}

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
        // 每次循环检查并更新最小间隔
        let guard = scheduler.lock().await;
        let new_min_interval = guard.get_min_interval();
        drop(guard); // 尽早释放锁

        // 如果最小间隔变化，重建ticker
        if new_min_interval != current_min_interval {
            current_min_interval = new_min_interval;
            ticker = Some(Ticker::every(current_min_interval));
            log::info!(
                "Updated scheduler ticker to {} ms",
                current_min_interval / 1000
            );
        }

        // 等待ticker触发（unwrap安全：上面已初始化）
        ticker.as_mut().unwrap().next().await;

        log::debug!("Scheduler tick - checking data sources");
        let mut guard = scheduler.lock().await;
        let now = Instant::now();

        // 遍历数据源，检查是否需要刷新
        for source_meta in guard.sources.iter_mut() {
            // 判断是否达到刷新时间
            if now - source_meta.last_refresh_tick >= source_meta.interval_tick {
                // 转换为毫秒
                log::debug!(
                    "[{:?}] Ready for refresh (now: {}, last: {}, interval: {})",
                    source_meta.id,
                    now,
                    source_meta.last_refresh_tick,
                    source_meta.interval_tick
                );

                // 执行刷新
                let mut source = source_meta.instance.lock().await;
                match source.refresh(system_api).await {
                    Ok(_) => {
                        source_meta.last_refresh_tick = now; // 更新上次刷新时间
                        // 更新最后更新时间
                        log::debug!("[{:?}] Refreshed successfully", source_meta.id);
                    }
                    Err(e) => {
                        log::warn!("[{:?}] Refresh failed: {:?}", source_meta.id, e);
                    }
                }
            }
        }

        drop(guard); // 释放锁

        // 所有数据源都更新完了，通知系统更新屏幕
        DISPLAY_EVENTS.send(DisplayEvent::FullRefresh).await;
    }
}
