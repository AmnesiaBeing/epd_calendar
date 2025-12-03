// src/tasks/display_task.rs
use embassy_time::{Duration, Instant, Timer};
use epd_waveshare::color::QuadColor;

use crate::common::error::{AppError, Result};
use crate::common::system_state::ChargingStatus;
use crate::common::{NetworkStatus, SystemState};
use crate::driver::display::DisplayDriver;
use crate::render::RenderEngine;
use crate::tasks::{ComponentData, DISPLAY_EVENTS, DisplayEvent};

// 配置常量
const FULL_REFRESH_INTERVAL_SECONDS: u64 = 15 * 60; // 15分钟强制全屏刷新
const MAX_PARTIAL_REFRESH_COUNT: u32 = 50; // 最大部分刷新次数
const REFRESH_RETRY_COUNT: u8 = 1; // 刷新重试次数

/// 显示任务主函数 - 负责协调屏幕渲染和刷新逻辑
///
/// 核心职责：
/// - 初始化显示驱动并处理可能的初始化失败，实现可靠启动
/// - 实现墨水屏特定的刷新策略（定期全屏刷新、部分刷新累积计数）
/// - 维护组件状态更新和错误处理机制
/// - 根据不同显示事件类型执行相应的刷新操作
#[embassy_executor::task]
pub async fn display_task(mut render_engine: RenderEngine) {
    log::info!("Display task started");

    // 初始化系统状态
    let mut system_state = SystemState::default();

    // 事件接收器和刷新计数管理
    let receiver = DISPLAY_EVENTS.receiver();
    let mut last_full_refresh = None;
    let mut partial_refresh_count = 0;

    // 首次启动时进行全局刷新
    if let Err(e) = perform_initial_full_refresh(
        &mut render_engine,
        &system_state,
        &mut last_full_refresh,
        &mut partial_refresh_count,
    )
    .await
    {
        log::error!("Initial full refresh failed: {:?}", e);
    }

    // 主事件循环 - 处理各种显示事件
    loop {
        match receiver.receive().await {
            DisplayEvent::FullRefresh => {
                handle_full_refresh(
                    &mut render_engine,
                    &system_state,
                    &mut last_full_refresh,
                    &mut partial_refresh_count,
                )
                .await;
            }

            DisplayEvent::UpdateComponent(component_data) => {
                handle_component_update(
                    &mut render_engine,
                    &mut system_state,
                    &component_data,
                    &mut last_full_refresh,
                    &mut partial_refresh_count,
                )
                .await;
            }

            // 处理其他类型的显示事件
            _ => {
                log::warn!("Unhandled display event");
            }
        }
    }
}

enum RefreshType<'a> {
    Full,
    Partial(&'a ComponentData),
}

/// 带延迟的重试执行函数
async fn retry_with_delay<F, T>(mut operation: F, delay: Duration, error_message: &str) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    match operation() {
        Ok(result) => Ok(result),
        Err(e) => {
            log::error!("{}: {:?}", error_message, e);
            Timer::after(delay).await;
            operation()
        }
    }
}

/// 执行带重试和错误处理的全屏刷新
async fn handle_full_refresh(
    render_engine: &mut RenderEngine,
    system_state: &SystemState,
    last_full_refresh: &mut Option<Instant>,
    partial_refresh_count: &mut u32,
) {
    if let Err(e) = perform_refresh_with_retry(
        render_engine,
        system_state,
        last_full_refresh,
        partial_refresh_count,
        RefreshType::Full,
    )
    .await
    {
        log::error!("Full refresh failed: {:?}", e);
        handle_refresh_failure(render_engine).await;
    }
}

/// 处理组件更新事件
async fn handle_component_update(
    render_engine: &mut RenderEngine,
    system_state: &mut SystemState,
    component_data: &ComponentData,
    last_full_refresh: &mut Option<Instant>,
    partial_refresh_count: &mut u32,
) {
    // 更新系统状态
    update_system_state(system_state, component_data);

    // 检查是否需要强制全屏刷新
    if should_force_full_refresh(*last_full_refresh, *partial_refresh_count) {
        log::info!("Forcing full refresh due to refresh count or schedule");
        handle_full_refresh(
            render_engine,
            system_state,
            last_full_refresh,
            partial_refresh_count,
        )
        .await;
        *partial_refresh_count = 0;
    } else {
        handle_partial_refresh(
            render_engine,
            system_state,
            component_data,
            last_full_refresh,
            partial_refresh_count,
        )
        .await;
    }
}

fn log_debug_partial_refresh_count(count: u32) {
    log::debug!(
        "Partial refresh count: {}/{}",
        count,
        MAX_PARTIAL_REFRESH_COUNT
    );
}

/// 处理部分刷新，包含重试和降级逻辑
async fn handle_partial_refresh(
    render_engine: &mut RenderEngine,
    system_state: &SystemState,
    component_data: &ComponentData,
    last_full_refresh: &mut Option<Instant>,
    partial_refresh_count: &mut u32,
) {
    // 执行部分刷新，失败时重试
    let result = perform_refresh_with_retry(
        render_engine,
        system_state,
        last_full_refresh,
        partial_refresh_count,
        RefreshType::Partial(component_data),
    )
    .await;

    match result {
        Ok(_) => {
            *partial_refresh_count += 1;
            log_debug_partial_refresh_count(*partial_refresh_count);
        }
        Err(e) => {
            log::error!("Partial refresh failed after retry: {:?}", e);
            // 降级到全屏刷新
            if let Err(full_err) = perform_full_refresh(
                render_engine,
                system_state,
                last_full_refresh,
                partial_refresh_count,
            ) {
                log::error!("Full refresh fallback failed: {:?}", full_err);
                handle_refresh_failure(render_engine).await;
            }
            *partial_refresh_count = 0;
        }
    }
}

/// 执行带重试的刷新操作
async fn perform_refresh_with_retry<'a>(
    render_engine: &mut RenderEngine,
    system_state: &SystemState,
    last_full_refresh: &mut Option<Instant>,
    partial_refresh_count: &mut u32,
    refresh_type: RefreshType<'a>,
) -> Result<()> {
    match refresh_type {
        RefreshType::Full => {
            retry_with_delay(
                || {
                    perform_full_refresh(
                        render_engine,
                        system_state,
                        last_full_refresh,
                        partial_refresh_count,
                    )
                },
                Duration::from_millis(500),
                "Refresh operation failed",
            )
            .await
        }
        RefreshType::Partial(component_data) => {
            retry_with_delay(
                || {
                    perform_partial_refresh(
                        render_engine,
                        system_state,
                        component_data,
                        partial_refresh_count,
                    )
                },
                Duration::from_millis(500),
                "Partial refresh failed",
            )
            .await
        }
    }
}

/// 执行首次全屏刷新
async fn perform_initial_full_refresh(
    render_engine: &mut RenderEngine,
    system_state: &SystemState,
    last_full_refresh: &mut Option<Instant>,
    partial_refresh_count: &mut u32,
) -> Result<()> {
    log::info!("Performing initial full refresh");

    retry_with_delay(
        || {
            perform_full_refresh(
                render_engine,
                system_state,
                last_full_refresh,
                partial_refresh_count,
            )
        },
        Duration::from_secs(5),
        "Initial full refresh failed",
    )
    .await
}

/// 更新系统状态
fn update_system_state(system_state: &mut SystemState, component_data: &ComponentData) {
    log::debug!("Updating system state component");

    match component_data {
        ComponentData::TimeData(data) => {
            system_state.time = Some(data.clone());
        }
        ComponentData::DateData(data) => {
            system_state.date = Some(data.clone());
        }
        ComponentData::WeatherData(data) => {
            system_state.weather = Some(data.clone());
        }
        ComponentData::QuoteData(data) => {
            system_state.quote = Some(data);
        }
        ComponentData::ChargingStatus(status) => {
            system_state.is_charging = status.clone();
        }
        ComponentData::BatteryData(battery_level) => {
            system_state.battery_level = *battery_level;
        }
        ComponentData::NetworkStatus(status) => {
            system_state.is_online = status.clone();
        }
        _ => {
            log::warn!("Mismatched component type and data");
        }
    }
}

/// 执行全屏刷新
fn perform_full_refresh(
    render_engine: &mut RenderEngine,
    system_state: &SystemState,
    last_full_refresh: &mut Option<Instant>,
    partial_refresh_count: &mut u32,
) -> Result<()> {
    log::info!("Performing full display refresh");

    render_engine.render_full_screen(system_state)?;

    // 更新最后全屏刷新时间
    *last_full_refresh = Some(Instant::now());
    *partial_refresh_count = 0;

    log::debug!("Full refresh completed successfully");
    Ok(())
}

/// 执行部分刷新
fn perform_partial_refresh(
    render_engine: &mut RenderEngine,
    system_state: &SystemState,
    component_data: &ComponentData,
    partial_refresh_count: &mut u32,
) -> Result<()> {
    log::debug!(
        "Performing partial refresh for component: {:?}",
        component_data
    );

    // 渲染需要更新的组件
    render_engine.render_component(component_data);

    *partial_refresh_count += 1;

    log::debug!("Partial refresh completed successfully");
    Ok(())
}

/// 判断是否需要强制全屏刷新
fn should_force_full_refresh(
    last_full_refresh: Option<Instant>,
    partial_refresh_count: u32,
) -> bool {
    // 检查部分刷新次数是否达到上限
    if partial_refresh_count >= MAX_PARTIAL_REFRESH_COUNT {
        log::debug!("Force full refresh due to partial refresh count limit reached");
        return true;
    }

    // 检查是否超过了全屏刷新时间间隔
    if let Some(last) = last_full_refresh {
        let elapsed = Instant::now() - last;
        if elapsed.as_secs() >= FULL_REFRESH_INTERVAL_SECONDS {
            log::debug!("Force full refresh due to time interval exceeded");
            return true;
        }
    } else {
        // 如果从未执行过全屏刷新，则强制刷新
        return true;
    }

    false
}

/// 处理刷新失败
async fn handle_refresh_failure(_render_engine: &mut RenderEngine) {
    log::error!("Handling display refresh failure");

    // TODO: 重启驱动

    // 等待显示稳定
    Timer::after(Duration::from_secs(1)).await;
}
