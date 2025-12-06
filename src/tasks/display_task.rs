// src/tasks/display_task.rs

//! 显示任务模块 - 处理屏幕显示和刷新逻辑
//! 
//! 该模块负责管理屏幕显示，包括组件渲染、屏幕刷新和防抖控制。

use embassy_time::{Duration, Instant, Timer};

use crate::common::SystemState;
use crate::common::error::Result;
use crate::render::RenderEngine;
use crate::tasks::{ComponentDataType, DISPLAY_EVENTS, DisplayEvent};

// 配置常量
const DEBOUNCE_INTERVAL_SECONDS: u64 = 60; // 1分钟防抖限制
const SCREEN_SLEEP_DELAY_MS: u64 = 2000; // 刷新后休眠延迟

/// 显示任务主函数
#[embassy_executor::task]
pub async fn display_task(mut render_engine: RenderEngine) {
    log::info!("🖥️ Display task started");

    // 初始化系统状态
    let mut system_state = SystemState::default();
    let mut last_refresh_time: Option<Instant> = None;
    let receiver = DISPLAY_EVENTS.receiver();

    // 初始全屏渲染并刷新
    log::info!("Performing initial display setup");

    // 渲染初始内容到内存缓冲区
    if let Err(e) = render_engine.render_full_screen(&system_state) {
        log::error!("Initial render failed: {:?}", e);
    } else {
        // 首次刷新显示
        if let Err(e) = render_engine.refresh_display().await {
            log::error!("Initial display refresh failed: {:?}", e);
        } else {
            last_refresh_time = Some(Instant::now());
            log::info!("Initial display setup completed");

            // 首次刷新后休眠屏幕
            Timer::after(Duration::from_millis(SCREEN_SLEEP_DELAY_MS)).await;
            if let Err(e) = render_engine.sleep_driver() {
                log::warn!("Failed to sleep display after initial setup: {:?}", e);
            }
        }
    }

    // 主事件循环
    loop {
        match receiver.receive().await {
            DisplayEvent::UpdateComponent(component_data) => {
                handle_update_component(
                    &mut render_engine,
                    &mut system_state,
                    &mut last_refresh_time,
                    &component_data,
                )
                .await;
            }

            DisplayEvent::ForceFullRefresh => {
                log::info!("Force full refresh requested");
                // 强制刷新忽略防抖限制
                if let Err(e) = execute_screen_refresh(
                    &mut render_engine,
                    &mut last_refresh_time,
                    true, // 强制刷新
                )
                .await
                {
                    log::error!("Force full refresh failed: {:?}", e);
                }
            }
        }
    }
}

/// 处理组件更新
/// 
/// # 参数
/// - `render_engine`: 渲染引擎实例
/// - `system_state`: 系统状态实例
/// - `last_refresh_time`: 上次刷新时间
/// - `component_data`: 组件数据
async fn handle_update_component(
    render_engine: &mut RenderEngine,
    system_state: &mut SystemState,
    last_refresh_time: &mut Option<Instant>,
    component_data: &ComponentDataType,
) {
    log::debug!("Processing component update: {:?}", component_data);

    // 1. 更新系统状态
    update_system_state(system_state, component_data);

    // 2. 更新内存缓冲区（只更新对应的组件）
    if let Err(e) = render_engine.render_component(component_data) {
        log::error!("Failed to render component to buffer: {:?}", e);
        return;
    }

    // 3. 检查是否需要刷新屏幕
    // 只有时间更新才考虑触发屏幕刷新
    if let ComponentDataType::TimeType(_) = component_data {
        if should_refresh_screen(*last_refresh_time) {
            log::info!("Time update triggers screen refresh");
            if let Err(e) = execute_screen_refresh(
                render_engine,
                last_refresh_time,
                false, // 非强制刷新
            )
            .await
            {
                log::error!("Screen refresh failed: {:?}", e);
            }
        } else {
            log::debug!("Screen refresh debounced, only updated memory buffer");
        }
    } else {
        // 非时间组件更新：只更新内存缓冲区，不刷新屏幕
        log::debug!("Non-time component updated, screen refresh deferred to next time update");
    }
}

/// 检查是否应该刷新屏幕（防抖检查）
/// 
/// # 参数
/// - `last_refresh_time`: 上次刷新时间
/// 
/// # 返回值
/// - `bool`: true表示应该刷新屏幕
fn should_refresh_screen(last_refresh_time: Option<Instant>) -> bool {
    match last_refresh_time {
        Some(last) => {
            let elapsed = Instant::now() - last;
            if elapsed.as_secs() >= DEBOUNCE_INTERVAL_SECONDS {
                log::debug!("Should refresh: {}s since last refresh", elapsed.as_secs());
                true
            } else {
                log::debug!(
                    "Refresh debounced: {}s since last refresh",
                    elapsed.as_secs()
                );
                false
            }
        }
        None => {
            // 从未刷新过，需要刷新
            log::debug!("Should refresh: never refreshed before");
            true
        }
    }
}

/// 执行屏幕刷新（将内存缓冲区传输到屏幕并显示）
/// 
/// # 参数
/// - `render_engine`: 渲染引擎实例
/// - `last_refresh_time`: 上次刷新时间
/// - `force_refresh`: 是否强制刷新
/// 
/// # 返回值
/// - `Result<()>`: 刷新成功返回Ok(()), 失败返回错误
async fn execute_screen_refresh(
    render_engine: &mut RenderEngine,
    last_refresh_time: &mut Option<Instant>,
    force_refresh: bool,
) -> Result<()> {
    // 防抖检查（除非是强制刷新）
    if !force_refresh && !should_refresh_screen(*last_refresh_time) {
        log::info!("Refresh skipped due to debounce");
        return Ok(());
    }

    log::info!("Executing screen refresh");

    // 刷新显示（将内存缓冲区传输到屏幕并更新显示）
    render_engine.refresh_display().await?;

    // 记录刷新时间
    *last_refresh_time = Some(Instant::now());
    log::debug!("Screen refresh completed, time recorded");

    // 延迟后休眠屏幕
    Timer::after(Duration::from_millis(SCREEN_SLEEP_DELAY_MS)).await;
    render_engine.sleep_driver()?;

    log::info!("Screen refreshed and put to sleep");
    Ok(())
}

/// 更新系统状态
/// 
/// # 参数
/// - `system_state`: 系统状态实例
/// - `component_data`: 组件数据
fn update_system_state(system_state: &mut SystemState, component_data: &ComponentDataType) {
    match component_data {
        ComponentDataType::TimeType(data) => {
            system_state.time = data.clone();
            log::debug!("Updated time component");
        }
        ComponentDataType::DateType(data) => {
            system_state.date = data.clone();
            log::debug!("Updated date component");
        }
        ComponentDataType::WeatherType(data) => {
            system_state.weather = data.clone();
            log::debug!("Updated weather component");
        }
        ComponentDataType::QuoteType(data) => {
            // 这里本身就是指针，直接引用即可
            system_state.quote = *data;
            log::debug!("Updated quote component");
        }
        ComponentDataType::ChargingStatusType(status) => {
            system_state.charging_status = status.clone();
            log::debug!("Updated charging status");
        }
        ComponentDataType::BatteryType(battery_level) => {
            system_state.battery_level = *battery_level;
            log::debug!("Updated battery level");
        }
        ComponentDataType::NetworkStatusType(status) => {
            system_state.network_status = status.clone();
            log::debug!("Updated network status");
        }
    }
}