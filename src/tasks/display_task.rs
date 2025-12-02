// src/tasks/display_task.rs
use crate::common::error::{AppError, Result};
use crate::common::system_state::{SYSTEM_STATE, SystemState};
use crate::render::RenderEngine;
use crate::render::components::{
    date_component::DateComponent, quote_component::QuoteComponent,
    status_component::StatusComponent, time_component::TimeComponent,
    weather_component::WeatherComponent,
};
use crate::tasks::{
    ComponentData, ComponentType, DISPLAY_EVENTS, DisplayEvent, PartialRefreshType,
};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::primitives::Rectangle;

// 配置常量
const FULL_REFRESH_INTERVAL_SECONDS: u64 = 15 * 60; // 15分钟强制全屏刷新
const MAX_PARTIAL_REFRESH_COUNT: usize = 50; // 最大部分刷新次数
const REFRESH_RETRY_COUNT: u8 = 1; // 刷新重试次数（根据需求设置为1次）

/// 显示任务主函数 - 负责协调屏幕渲染和刷新逻辑
#[embassy_executor::task]
pub async fn display_task(mut render_engine: RenderEngine) {
    log::info!("Display task started");

    // 初始化显示驱动 - 由于display_driver是私有字段，我们假设它已经在RenderEngine构造时初始化
    log::info!("Display driver initialized successfully");

    let receiver = DISPLAY_EVENTS.receiver();
    let mut last_full_refresh: Option<Instant> = None;
    let mut partial_refresh_count: usize = 0;

    // 首次启动时进行全局刷新，确保所有组件内容正确显示
    log::info!("Performing initial full refresh");
    if let Err(e) = perform_full_refresh(&mut render_engine, &mut last_full_refresh).await {
        log::error!("Initial full refresh failed: {:?}", e);
        // 等待一段时间后重试
        Timer::after(Duration::from_secs(5)).await;
        if let Err(e) = perform_full_refresh(&mut render_engine, &mut last_full_refresh).await {
            log::error!("Initial full refresh failed again: {:?}", e);
        }
    }
    partial_refresh_count = 0;

    // 主事件循环 - 处理各种显示事件
    loop {
        match receiver.receive().await {
            DisplayEvent::FullRefresh => {
                // 直接执行全屏刷新
                if let Err(e) =
                    perform_full_refresh(&mut render_engine, &mut last_full_refresh).await
                {
                    log::error!("Full refresh failed: {:?}", e);
                    handle_refresh_failure(&mut render_engine).await;
                }
                partial_refresh_count = 0;
            }

            DisplayEvent::PartialRefresh(refresh_type) => {
                // 检查是否需要强制全屏刷新（基于时间或次数限制）
                if should_force_full_refresh(last_full_refresh, partial_refresh_count) {
                    log::info!("Forcing full refresh due to refresh count or schedule");
                    if let Err(e) =
                        perform_full_refresh(&mut render_engine, &mut last_full_refresh).await
                    {
                        log::error!("Scheduled full refresh failed: {:?}", e);
                        handle_refresh_failure(&mut render_engine).await;
                    }
                    partial_refresh_count = 0;
                } else {
                    // 执行部分刷新
                    if let Err(e) = perform_partial_refresh(&mut render_engine, &refresh_type).await
                    {
                        log::error!("Partial refresh failed: {:?}", e);
                        // 降级到全屏刷新
                        if let Err(retry_err) =
                            perform_full_refresh(&mut render_engine, &mut last_full_refresh).await
                        {
                            log::error!("Full refresh fallback failed: {:?}", retry_err);
                            handle_refresh_failure(&mut render_engine).await;
                        }
                        partial_refresh_count = 0;
                    } else {
                        partial_refresh_count += 1;
                        log::debug!(
                            "Partial refresh count: {}/{}",
                            partial_refresh_count,
                            MAX_PARTIAL_REFRESH_COUNT
                        );
                    }
                }
            }

            DisplayEvent::UpdateComponent(component_type, component_data) => {
                // 处理组件更新，包括更新系统状态和屏幕刷新
                if let Err(e) = handle_component_update(
                    &mut render_engine,
                    component_type,
                    component_data,
                    &mut last_full_refresh,
                    &mut partial_refresh_count,
                )
                .await
                {
                    log::error!("Component update failed: {:?}", e);
                    // 降级到全屏刷新
                    if let Err(retry_err) =
                        perform_full_refresh(&mut render_engine, &mut last_full_refresh).await
                    {
                        log::error!("Full refresh fallback failed: {:?}", retry_err);
                        handle_refresh_failure(&mut render_engine).await;
                    }
                    partial_refresh_count = 0;
                }
            }

            DisplayEvent::RequestLunarCalc => {
                // 处理农历计算请求
                handle_lunar_calculation().await;
            }
        }
    }
}

/// 执行全屏刷新
/// 功能：刷新整个屏幕，清除之前的所有内容并重新绘制所有组件
async fn perform_full_refresh(
    render_engine: &mut RenderEngine,
    last_full_refresh: &mut Option<Instant>,
) -> Result<()> {
    log::info!("Performing full display refresh");

    // 更新最后全屏刷新时间戳
    *last_full_refresh = Some(Instant::now());

    // 获取系统状态 - 先获取StaticCell的值，再使用lock方法
    let system_state = SYSTEM_STATE.get_or_init(|| GlobalMutex::new(SystemState::default())).lock().await;

    // 清空缓冲区，准备重新绘制
    render_engine.clear_buffer();

    // 创建并渲染所有组件
    render_all_components(&system_state, render_engine)?;

    // 执行全屏刷新
    match render_engine
        .display_driver
        .update_frame(render_engine.display_buffer.buffer())
    {
        Ok(_) => match render_engine.display_driver.display_frame() {
            Ok(_) => {
                log::info!("Full refresh completed successfully");
                Ok(())
            }
            Err(_) => {
                log::error!("Failed to display frame during full refresh");
                Err(AppError::DisplayFullRefreshFailed)
            }
        },
        Err(_) => {
            log::error!("Failed to update frame during full refresh");
            Err(AppError::DisplayFullRefreshFailed)
        }
    }
}

/// 执行部分刷新
/// 功能：只刷新指定区域的内容，提高刷新效率和减少屏幕闪烁
async fn perform_partial_refresh(
    render_engine: &mut RenderEngine,
    refresh_type: &PartialRefreshType,
) -> Result<()> {
    log::debug!("Performing partial refresh: {:?}", refresh_type);

    // 获取系统状态 - 先获取StaticCell的值，再使用lock方法
    let system_state = SYSTEM_STATE.get_or_init(|| GlobalMutex::new(SystemState::default())).lock().await;

    // 清除之前的脏区域，准备新的部分刷新
    render_engine.clear_dirty();

    // 根据刷新类型渲染相应组件
    match render_partial_components_by_type(&system_state, render_engine, refresh_type) {
        Ok(_) => {
            // 执行部分刷新
            match render_engine.partial_refresh().await {
                Ok(_) => {
                    log::debug!("Partial refresh completed successfully");
                    Ok(())
                }
                Err(e) => {
                    log::error!("Partial refresh failed during display: {:?}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            log::error!("Failed to render components for partial refresh: {:?}", e);
            Err(e)
        }
    }
}

/// 处理组件更新
/// 功能：更新系统状态并根据需要刷新相应组件
async fn handle_component_update(
    render_engine: &mut RenderEngine,
    component_type: ComponentType,
    component_data: ComponentData,
    last_full_refresh: &mut Option<Instant>,
    partial_refresh_count: &mut usize,
) -> Result<()> {
    log::debug!("Updating component: {:?}", component_type);

    // 更新系统状态
    {
        let mut system_state = SYSTEM_STATE.lock().await;
        update_system_state_with_component_data(&mut system_state, component_type, component_data)?;
    }

    // 检查是否需要强制全屏刷新
    if should_force_full_refresh(*last_full_refresh, *partial_refresh_count) {
        log::info!("Scheduled full refresh triggered during component update");
        return perform_full_refresh(render_engine, last_full_refresh).await;
    }

    // 清除脏区域
    render_engine.clear_dirty();

    // 获取系统状态 - 先获取StaticCell的值，再使用lock方法
    let system_state = SYSTEM_STATE.get_or_init(|| GlobalMutex::new(SystemState::default())).lock().await;
    match render_updated_component(&system_state, render_engine, component_type) {
        Ok(_) => {
            // 执行部分刷新
            match render_engine.partial_refresh().await {
                Ok(_) => {
                    // 更新部分刷新计数
                    *partial_refresh_count += 1;
                    log::debug!(
                        "Component update completed, partial refresh count: {}/{}",
                        *partial_refresh_count,
                        MAX_PARTIAL_REFRESH_COUNT
                    );
                    Ok(())
                }
                Err(e) => {
                    log::error!("Partial refresh failed after component update: {:?}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            log::error!("Failed to render updated component: {:?}", e);
            Err(e)
        }
    }
}

/// 使用组件数据更新系统状态
fn update_system_state_with_component_data(
    system_state: &mut SystemState,
    component_type: ComponentType,
    component_data: ComponentData,
) -> Result<()> {
    match (component_type, component_data) {
        (ComponentType::Time, ComponentData::TimeData(time_data)) => {
            system_state.time = Some(time_data);
        }
        (ComponentType::Date, ComponentData::DateData(date_data)) => {
            system_state.date = Some(date_data);
        }
        (ComponentType::Weather, ComponentData::WeatherData(weather_data)) => {
            system_state.weather = Some(weather_data);
        }
        (ComponentType::Quote, ComponentData::QuoteData(quote_data)) => {
            system_state.quote = Some(quote_data);
        }
        (ComponentType::Battery, ComponentData::BatteryData(battery_data)) => {
            system_state.battery_level = battery_data;
        }
        (ComponentType::Network, ComponentData::NetworkStatus(network_status)) => {
            system_state.is_online = network_status;
        }
        (_, _) => {
            return Err(AppError::InvalidComponentData(format!(
                "Mismatch component type and data: {:?}, {:?}",
                component_type, component_data
            )));
        }
    }
    Ok(())
}

/// 处理农历计算
async fn handle_lunar_calculation() {
    log::debug!("Handling lunar calculation request");

    // 获取当前系统状态并更新农历信息
    let system_state = SYSTEM_STATE.get_or_init(|| GlobalMutex::new(SystemState::default())).lock().await;
    if let Err(e) = system_state.update_lunar_info() {
        log::error!("Failed to update lunar info: {:?}", e);
    }
    log::debug!("Lunar calculation completed");
}

/// 检查是否应该强制执行全屏刷新
fn should_force_full_refresh(
    last_full_refresh: Option<Instant>,
    partial_refresh_count: usize,
) -> bool {
    // 检查部分刷新次数是否达到限制
    if partial_refresh_count >= MAX_PARTIAL_REFRESH_COUNT {
        log::info!("Max partial refresh count reached");
        return true;
    }

    // 检查是否到了计划的全屏刷新时间
    if let Some(last_refresh) = last_full_refresh {
        if Instant::now() - last_refresh >= Duration::from_secs(FULL_REFRESH_INTERVAL_SECONDS) {
            log::info!("Scheduled full refresh time reached");
            return true;
        }
    } else {
        // 如果从未执行过全屏刷新，则应该执行
        return true;
    }

    false
}

/// 处理刷新失败
async fn handle_refresh_failure(render_engine: &mut RenderEngine) {
    log::error!("Entering refresh failure recovery mode");

    // 尝试重试
    for attempt in 1..=REFRESH_RETRY_COUNT {
        log::info!("Refresh retry attempt {}/{}", attempt, REFRESH_RETRY_COUNT);

        // 短暂延迟
        Timer::after(Duration::from_secs(attempt as u64 * 2)).await;

        // 尝试简单的全屏刷新进行恢复
        match perform_full_refresh_for_recovery(render_engine).await {
            Ok(_) => {
                log::info!("Refresh recovery successful on attempt {}", attempt);
                return;
            }
            Err(e) => {
                log::error!("Refresh recovery attempt {} failed: {:?}", attempt, e);
            }
        }
    }

    log::error!("All refresh recovery attempts failed");
    // 尝试将显示驱动唤醒，然后再次尝试
    if let Err(e) = render_engine.display_driver.wake_up() {
        log::error!("Failed to wake up display driver: {:?}", e);
    }
}

/// 执行恢复用的全屏刷新
async fn perform_full_refresh_for_recovery(render_engine: &mut RenderEngine) -> Result<()> {
    let system_state = SYSTEM_STATE.get_or_init(|| GlobalMutex::new(SystemState::default())).lock().await;

    // 清空缓冲区
    render_engine.clear_buffer();

    // 渲染所有组件
    render_all_components(&system_state, render_engine)?;

    // 执行全屏刷新
    render_engine
        .display_driver
        .update_frame(render_engine.display_buffer.buffer())?;
    render_engine.display_driver.display_frame()?;

    Ok(())
}

/// 渲染所有组件
fn render_all_components(
    system_state: &SystemState,
    render_engine: &mut RenderEngine,
) -> Result<()> {
    // 按顺序渲染所有组件
    if let Some(time_data) = &system_state.time {
        let time_component = TimeComponent::new(time_data.clone());
        if let Err(e) = time_component.draw(&mut render_engine.display_buffer) {
            log::error!("Failed to draw time component: {:?}", e);
            return Err(AppError::RenderingFailed);
        }
    }

    if let Some(date_data) = &system_state.date {
        let date_component = DateComponent::new(date_data.clone());
        if let Err(e) = date_component.draw(&mut render_engine.display_buffer) {
            log::error!("Failed to draw date component: {:?}", e);
            return Err(AppError::RenderingFailed);
        }
    }

    if let Some(weather_data) = &system_state.weather {
        let weather_component = WeatherComponent::new(weather_data.clone());
        if let Err(e) = weather_component.draw(&mut render_engine.display_buffer) {
            log::error!("Failed to draw weather component: {:?}", e);
            return Err(AppError::RenderingFailed);
        }
    }

    if let Some(quote_data) = &system_state.quote {
        let quote_component = QuoteComponent::new(*quote_data);
        if let Err(e) = quote_component.draw(&mut render_engine.display_buffer) {
            log::error!("Failed to draw quote component: {:?}", e);
            return Err(AppError::RenderingFailed);
        }
    }

    // 渲染状态组件（电池和网络）
    let status_component = StatusComponent::new(system_state.battery_level, system_state.is_online);
    if let Err(e) = status_component.draw(&mut render_engine.display_buffer) {
        log::error!("Failed to draw status component: {:?}", e);
        return Err(AppError::RenderingFailed);
    }

    Ok(())
}

/// 根据部分刷新类型渲染组件
fn render_partial_components_by_type(
    system_state: &SystemState,
    render_engine: &mut RenderEngine,
    refresh_type: &PartialRefreshType,
) -> Result<()> {
    match refresh_type {
        PartialRefreshType::TimeOnly => {
            if let Some(time_data) = &system_state.time {
                render_component_with_dirty_region(
                    render_engine,
                    &TimeComponent::new(time_data.clone()),
                    TimeComponent::get_bounding_box(),
                )?;
            }
        }
        PartialRefreshType::DateOnly => {
            if let Some(date_data) = &system_state.date {
                render_component_with_dirty_region(
                    render_engine,
                    &DateComponent::new(date_data.clone()),
                    DateComponent::get_bounding_box(),
                )?;
            }
        }
        PartialRefreshType::WeatherOnly => {
            if let Some(weather_data) = &system_state.weather {
                render_component_with_dirty_region(
                    render_engine,
                    &WeatherComponent::new(weather_data.clone()),
                    WeatherComponent::get_bounding_box(),
                )?;
            }
        }
        PartialRefreshType::QuoteOnly => {
            if let Some(quote_data) = &system_state.quote {
                render_component_with_dirty_region(
                    render_engine,
                    &QuoteComponent::new(*quote_data),
                    QuoteComponent::get_bounding_box(),
                )?;
            }
        }
        PartialRefreshType::StatusOnly => {
            render_component_with_dirty_region(
                render_engine,
                &StatusComponent::new(system_state.battery_level, system_state.is_online),
                StatusComponent::get_bounding_box(),
            )?;
        }
        PartialRefreshType::TimeAndDate => {
            // 渲染时间组件
            if let Some(time_data) = &system_state.time {
                render_component_with_dirty_region(
                    render_engine,
                    &TimeComponent::new(time_data.clone()),
                    TimeComponent::get_bounding_box(),
                )?;
            }
            // 渲染日期组件
            if let Some(date_data) = &system_state.date {
                render_component_with_dirty_region(
                    render_engine,
                    &DateComponent::new(date_data.clone()),
                    DateComponent::get_bounding_box(),
                )?;
            }
        }
    }
    Ok(())
}

/// 渲染更新的组件
fn render_updated_component(
    system_state: &SystemState,
    render_engine: &mut RenderEngine,
    component_type: ComponentType,
) -> Result<()> {
    match component_type {
        ComponentType::Time => {
            if let Some(time_data) = &system_state.time {
                render_component_with_dirty_region(
                    render_engine,
                    &TimeComponent::new(time_data.clone()),
                    TimeComponent::get_bounding_box(),
                )?;
            }
        }
        ComponentType::Date => {
            if let Some(date_data) = &system_state.date {
                render_component_with_dirty_region(
                    render_engine,
                    &DateComponent::new(date_data.clone()),
                    DateComponent::get_bounding_box(),
                )?;
            }
        }
        ComponentType::Weather => {
            if let Some(weather_data) = &system_state.weather {
                render_component_with_dirty_region(
                    render_engine,
                    &WeatherComponent::new(weather_data.clone()),
                    WeatherComponent::get_bounding_box(),
                )?;
            }
        }
        ComponentType::Quote => {
            if let Some(quote_data) = &system_state.quote {
                render_component_with_dirty_region(
                    render_engine,
                    &QuoteComponent::new(*quote_data),
                    QuoteComponent::get_bounding_box(),
                )?;
            }
        }
        ComponentType::Battery | ComponentType::Network => {
            render_component_with_dirty_region(
                render_engine,
                &StatusComponent::new(system_state.battery_level, system_state.is_online),
                StatusComponent::get_bounding_box(),
            )?;
        }
    }
    Ok(())
}

/// 渲染组件并标记脏区域
fn render_component_with_dirty_region<
    T: embedded_graphics::Drawable<Color = epd_waveshare::color::QuadColor>,
>(
    render_engine: &mut RenderEngine,
    component: &T,
    bounding_box: Rectangle,
) -> Result<()> {
    // 标记组件的脏区域
    render_engine.mark_dirty(bounding_box);

    // 绘制组件到缓冲区
    if let Err(e) = component.draw(&mut render_engine.display_buffer) {
        log::error!("Failed to draw component: {:?}", e);
        return Err(AppError::RenderingFailed);
    }

    Ok(())
}