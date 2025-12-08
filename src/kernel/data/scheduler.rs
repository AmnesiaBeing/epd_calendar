// src/kernel/data/scheduler.rs
//! 数据源调度器模块
//! 实现单静态任务的数据源调度，适配embassy"静态任务、无堆分配"特性

use crate::common::error::{AppError, Result};
use crate::common::{GlobalChannel, GlobalMutex};
use crate::kernel::data::source::DataSource;
use crate::kernel::data::types::DataSourceId;
use crate::kernel::system::api::SystemApi;

use embassy_time::{Duration, Ticker};
use heapless::Vec;

/// 数据源更新事件
pub enum DataSourceEvent {
    /// 数据源已更新
    Updated(DataSourceId),
    /// 数据源更新失败
    Failed(DataSourceId, AppError),
}

/// 数据源元数据
pub struct SourceMeta {
    /// 数据源ID
    pub id: DataSourceId,
    /// 数据源实例
    pub instance: &'static GlobalMutex<dyn DataSource>,
    /// 刷新间隔（秒）
    pub interval_secs: u64,
    /// 上次刷新时间（系统ticks）
    pub last_refresh_tick: u64,
}

/// 数据源调度器
/// 管理所有数据源的定时刷新，适配embassy静态任务特性
pub struct DataSourceScheduler {
    /// 数据源元数据列表
    pub sources: Vec<SourceMeta, 8>,
    /// 最小刷新间隔（秒）
    pub min_interval: u64,
}

impl Default for DataSourceScheduler {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            min_interval: 60, // 默认最小间隔60秒
        }
    }
}

impl DataSourceScheduler {
    /// 创建新的数据源调度器
    pub const fn new() -> Self {
        Self {
            sources: Vec::new(),
            min_interval: 60,
        }
    }

    /// 静态注册数据源
    pub fn register_source(
        &mut self,
        id: DataSourceId,
        instance: &'static GlobalMutex<dyn DataSource>,
        interval_secs: u32,
    ) -> Result<()> {
        // 检查数据源是否已注册
        if self.sources.iter().any(|s| s.id == id) {
            log::warn!("DataSource {:?} already registered", id);
            return Ok(());
        }

        // 添加数据源元数据
        self.sources.push(SourceMeta {
            id,
            instance,
            interval_secs: interval_secs as u64,
            last_refresh_tick: 0,
        })?;

        // 重新计算最小刷新间隔
        self.min_interval = self.get_min_interval();
        log::info!(
            "Registered DataSource {:?}, interval: {}s, new min interval: {}s",
            id,
            interval_secs,
            self.min_interval
        );

        Ok(())
    }

    /// 计算所有数据源的最小刷新间隔
    pub fn get_min_interval(&self) -> u64 {
        if self.sources.is_empty() {
            return 60; // 默认60秒
        }

        // 取所有数据源间隔的最小值
        self.sources
            .iter()
            .map(|s| s.interval_secs)
            .min()
            .unwrap_or(60)
    }

    /// 手动触发指定数据源的刷新
    pub async fn refresh_source(
        &mut self,
        id: DataSourceId,
        system_api: &dyn SystemApi,
        event_chan: &GlobalChannel<DataSourceEvent, 8>,
    ) -> Result<()> {
        // 查找数据源
        if let Some(source_meta) = self.sources.iter_mut().find(|s| s.id == id) {
            // 执行刷新
            let mut source = source_meta.instance.lock().await;
            if source.refresh(system_api).await.is_ok() {
                let now = system_api.get_hardware_api().get_system_ticks();
                source_meta.last_refresh_tick = now;
                event_chan.send(DataSourceEvent::Updated(id)).await.ok();
                log::debug!("[{}] Manually refreshed", id);
            }
        }

        Ok(())
    }

    /// 刷新所有数据源
    pub async fn refresh_all(
        &mut self,
        system_api: &dyn SystemApi,
        event_chan: &GlobalChannel<DataSourceEvent, 8>,
    ) -> Result<()> {
        let now = system_api.get_hardware_api().get_system_ticks();

        for source_meta in self.sources.iter_mut() {
            // 执行刷新
            let mut source = source_meta.instance.lock().await;
            if source.refresh(system_api).await.is_ok() {
                source_meta.last_refresh_tick = now;
                event_chan
                    .send(DataSourceEvent::Updated(source_meta.id))
                    .await
                    .ok();
                log::debug!("[{}] Refreshed in refresh_all", source_meta.id);
            }
        }

        Ok(())
    }
}

/// 单静态任务：统一轮询所有数据源
/// 按所有数据源的最小刷新间隔定时执行，遍历检查各数据源是否达到刷新时间
#[embassy_executor::task]
pub async fn generic_scheduler_task(
    scheduler: &'static GlobalMutex<DataSourceScheduler>,
    system_api: &'static dyn SystemApi,
    event_chan: &'static GlobalChannel<DataSourceEvent>,
) {
    log::info!("Starting DataSource scheduler task");

    // 初始化：获取最小刷新间隔
    let min_interval = scheduler.lock().await.get_min_interval();
    let mut ticker = Ticker::every(Duration::from_secs(min_interval));

    log::info!("Scheduler ticker set to {} seconds", min_interval);

    loop {
        ticker.next().await; // 低功耗休眠，到点唤醒

        log::debug!("Scheduler tick - checking data sources");
        let mut guard = scheduler.lock().await;
        let now = system_api.get_hardware_api().get_system_ticks();

        // 遍历数据源，检查是否需要刷新
        for source_meta in guard.sources.iter_mut() {
            // 判断是否达到刷新时间
            if now - source_meta.last_refresh_tick >= source_meta.interval_secs * 1000 {
                // 转换为毫秒
                log::debug!(
                    "[{:?}] Ready for refresh (now: {}, last: {}, interval: {})
",
                    source_meta.id,
                    now,
                    source_meta.last_refresh_tick,
                    source_meta.interval_secs
                );

                // 执行刷新
                let mut source = source_meta.instance.lock().await;
                match source.refresh(system_api).await {
                    Ok(_) => {
                        source_meta.last_refresh_tick = now; // 更新上次刷新时间
                        event_chan
                            .send(DataSourceEvent::Updated(source_meta.id))
                            .await;
                        log::debug!("[{:?}] Refreshed successfully", source_meta.id);
                    }
                    Err(e) => {
                        log::warn!("[{:?}] Refresh failed: {:?}", source_meta.id, e);
                        event_chan
                            .send(DataSourceEvent::Failed(source_meta.id, e))
                            .await;
                    }
                }
            }
        }

        drop(guard); // 释放锁
    }
}
