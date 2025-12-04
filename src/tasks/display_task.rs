// src/tasks/display_task.rs
use embassy_time::{Duration, Instant, Timer};

use crate::common::SystemState;
use crate::common::error::{AppError, Result};
use crate::render::RenderEngine;
use crate::tasks::{ComponentData, DISPLAY_EVENTS, DisplayEvent};

// 配置常量
const FULL_REFRESH_INTERVAL_SECONDS: u64 = 15 * 60; // 15分钟强制全屏刷新
const MAX_PARTIAL_REFRESH_COUNT: u32 = 50; // 最大部分刷新次数
const RETRY_DELAY_MS: u64 = 500; // 重试延迟
const INITIAL_RETRY_DELAY_MS: u64 = 5000; // 初始重试延迟

/// 刷新策略管理器
struct RefreshStrategy {
    last_full_refresh: Option<Instant>,
    partial_refresh_count: u32,
}

impl RefreshStrategy {
    fn new() -> Self {
        Self {
            last_full_refresh: None,
            partial_refresh_count: 0,
        }
    }

    /// 检查是否需要强制全屏刷新
    fn should_force_full_refresh(&self) -> bool {
        // 从未执行过全屏刷新
        if self.last_full_refresh.is_none() {
            log::debug!("Force full refresh: never performed before");
            return true;
        }

        // 检查部分刷新次数是否达到上限
        if self.partial_refresh_count >= MAX_PARTIAL_REFRESH_COUNT {
            log::debug!(
                "Force full refresh: partial refresh count {} reached limit {}",
                self.partial_refresh_count,
                MAX_PARTIAL_REFRESH_COUNT
            );
            return true;
        }

        // 检查是否超过了全屏刷新时间间隔
        if let Some(last) = self.last_full_refresh {
            let elapsed = Instant::now() - last;
            if elapsed.as_secs() >= FULL_REFRESH_INTERVAL_SECONDS {
                log::debug!("Force full refresh: time interval exceeded");
                return true;
            }
        }

        false
    }

    /// 记录全屏刷新完成
    fn record_full_refresh(&mut self) {
        self.last_full_refresh = Some(Instant::now());
        self.partial_refresh_count = 0;
        log::debug!("Full refresh recorded, resetting partial count");
    }

    /// 记录部分刷新完成
    fn record_partial_refresh(&mut self) {
        self.partial_refresh_count += 1;
        log::debug!(
            "Partial refresh count: {}/{}",
            self.partial_refresh_count,
            MAX_PARTIAL_REFRESH_COUNT
        );
    }
}

/// 显示任务主函数
///
/// 简化职责：
/// - 初始化显示驱动
/// - 管理墨水屏刷新策略（定期全刷、累积计数）
/// - 处理组件更新事件
/// - 实现错误恢复机制
#[embassy_executor::task]
pub async fn display_task(mut render_engine: RenderEngine) {
    log::info!("🖥️ Display task started");

    // 初始化系统状态
    let mut system_state = SystemState::default();
    let mut refresh_strategy = RefreshStrategy::new();
    let receiver = DISPLAY_EVENTS.receiver();

    // 执行初始全屏刷新
    if let Err(e) = initialize_display(&mut render_engine, &system_state).await {
        log::error!("Failed to initialize display: {:?}", e);
        // 继续运行，尝试在后续恢复
    } else {
        refresh_strategy.record_full_refresh();
    }

    // 主事件循环
    loop {
        match receiver.receive().await {
            DisplayEvent::UpdateComponent(component_data) => {
                handle_update_component(
                    &mut render_engine,
                    &mut system_state,
                    &mut refresh_strategy,
                    &component_data,
                )
                .await;
            }

            DisplayEvent::ForceFullRefresh => {
                log::info!("Force full refresh requested");
                if let Err(e) =
                    execute_full_refresh(&mut render_engine, &system_state, &mut refresh_strategy)
                        .await
                {
                    log::error!("Force full refresh failed: {:?}", e);
                }
            }
        }
    }
}

/// 初始化显示（包含重试机制）
async fn initialize_display(
    render_engine: &mut RenderEngine,
    system_state: &SystemState,
) -> Result<()> {
    log::info!("Initializing display with full refresh");

    // 最多重试3次
    for attempt in 1..=3 {
        match render_engine.render_full_screen(system_state) {
            Ok(()) => {
                log::info!("Display initialized successfully");
                return Ok(());
            }
            Err(e) => {
                log::warn!("Initialization attempt {} failed: {:?}", attempt, e);
                if attempt < 3 {
                    Timer::after(Duration::from_millis(INITIAL_RETRY_DELAY_MS)).await;
                }
            }
        }
    }

    Err(AppError::DisplayInit)
}

/// 处理组件更新
async fn handle_update_component(
    render_engine: &mut RenderEngine,
    system_state: &mut SystemState,
    refresh_strategy: &mut RefreshStrategy,
    component_data: &ComponentData,
) {
    log::debug!("Processing component update: {:?}", component_data);

    // 1. 更新系统状态
    update_system_state(system_state, component_data);

    // 2. 检查是否需要全屏刷新
    if refresh_strategy.should_force_full_refresh() {
        log::info!("Performing scheduled full refresh");
        if let Err(e) = execute_full_refresh(render_engine, system_state, refresh_strategy).await {
            log::error!("Scheduled full refresh failed: {:?}", e);
            return;
        }
    } else {
        // 3. 尝试部分刷新
        if let Err(e) = execute_partial_refresh(render_engine, component_data).await {
            log::warn!("Partial refresh failed, falling back to full: {:?}", e);

            // 降级到全屏刷新
            if let Err(e) =
                execute_full_refresh(render_engine, system_state, refresh_strategy).await
            {
                log::error!("Fallback full refresh also failed: {:?}", e);
                return;
            }
        } else {
            // 部分刷新成功，记录
            refresh_strategy.record_partial_refresh();
        }
    }
}

/// 执行全屏刷新（带重试）
async fn execute_full_refresh(
    render_engine: &mut RenderEngine,
    system_state: &SystemState,
    refresh_strategy: &mut RefreshStrategy,
) -> Result<()> {
    log::info!("Executing full screen refresh");

    // 重试机制
    for attempt in 0..2 {
        // 最多重试1次（共2次尝试）
        match render_engine.render_full_screen(system_state) {
            Ok(()) => {
                refresh_strategy.record_full_refresh();
                log::debug!("Full refresh completed successfully");
                return Ok(());
            }
            Err(e) => {
                log::warn!("Full refresh attempt {} failed: {:?}", attempt + 1, e);
                if attempt < 1 {
                    Timer::after(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
        }
    }

    // 如果所有重试都失败，尝试恢复显示
    recover_display(render_engine).await;
    Err(AppError::DisplayFullRefreshFailed)
}

/// 执行部分刷新（带重试）
async fn execute_partial_refresh(
    render_engine: &mut RenderEngine,
    component_data: &ComponentData,
) -> Result<()> {
    log::debug!("Executing partial refresh for component");

    // 重试机制
    for attempt in 0..2 {
        // 最多重试1次
        match render_engine.render_component(component_data) {
            Ok(()) => {
                log::debug!("Partial refresh completed");
                return Ok(());
            }
            Err(e) => {
                log::warn!("Partial refresh attempt {} failed: {:?}", attempt + 1, e);
                if attempt < 1 {
                    Timer::after(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
        }
    }

    Err(AppError::DisplayPartialRefreshFailed)
}

/// 更新系统状态
fn update_system_state(system_state: &mut SystemState, component_data: &ComponentData) {
    match component_data {
        ComponentData::TimeData(data) => {
            system_state.time = Some(data.clone());
            log::debug!("Updated time component");
        }
        ComponentData::DateData(data) => {
            system_state.date = Some(data.clone());
            log::debug!("Updated date component");
        }
        ComponentData::WeatherData(data) => {
            system_state.weather = Some(data.clone());
            log::debug!("Updated weather component");
        }
        ComponentData::QuoteData(data) => {
            // 这里本身就是指针，直接引用即可
            system_state.quote = Some(*data);
            log::debug!("Updated quote component");
        }
        ComponentData::ChargingStatus(status) => {
            system_state.is_charging = status.clone();
            log::debug!("Updated charging status");
        }
        ComponentData::BatteryData(battery_level) => {
            system_state.battery_level = *battery_level;
            log::debug!("Updated battery level");
        }
        ComponentData::NetworkStatus(status) => {
            system_state.is_online = status.clone();
            log::debug!("Updated network status");
        }
    }
}

/// 恢复显示（硬件级恢复）
async fn recover_display(_render_engine: &mut RenderEngine) {
    log::warn!("Attempting display recovery");

    // 1. 短暂延迟让显示稳定
    Timer::after(Duration::from_millis(100)).await;

    // 2. 尝试重置显示驱动（如果支持）
    // 注意：这里假设 RenderEngine 有 reset 方法
    // 实际实现需要根据具体的显示驱动调整

    // 3. 记录恢复尝试
    log::info!("Display recovery sequence completed");
}
