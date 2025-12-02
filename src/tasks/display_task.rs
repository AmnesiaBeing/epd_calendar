// src/tasks/display_task.rs
use embassy_time::{Duration, Instant};

use crate::common::SystemState;
use crate::common::system_state::SYSTEM_STATE;
use crate::render::RenderEngine;
use crate::tasks::{
    ComponentData, ComponentType, DISPLAY_EVENTS, DisplayEvent, PartialRefreshType,
};

/// 显示任务主函数
#[embassy_executor::task]
pub async fn display_task(mut render_engine: RenderEngine) {
    log::info!("Display task started");

    let receiver = DISPLAY_EVENTS.receiver();
    let mut last_full_refresh: Option<Instant> = None;

    // 首次启动时进行全局刷新
    handle_full_refresh(&mut render_engine, &mut last_full_refresh).await;

    // 主事件循环
    loop {
        match receiver.receive().await {
            DisplayEvent::FullRefresh => {
                handle_full_refresh(&mut render_engine, &mut last_full_refresh).await;
            }

            DisplayEvent::PartialRefresh(refresh_type) => {
                handle_partial_refresh(&mut render_engine, &refresh_type).await;
            }

            DisplayEvent::UpdateComponent(component_type, component_data) => {
                handle_component_update(
                    &mut render_engine,
                    component_type,
                    component_data,
                    &mut last_full_refresh,
                )
                .await;
            }

            DisplayEvent::RequestLunarCalc => {
                handle_lunar_request(&SYSTEM_STATE).await;
            }
        }
    }
}

/// 处理全局刷新
async fn handle_full_refresh(
    render_engine: &mut RenderEngine,
    last_full_refresh: &mut Option<Instant>,
) {
    log::info!("Performing full display refresh");

    // 更新时间戳
    *last_full_refresh = Some(Instant::now());

    // 渲染所有组件到缓冲区
    if let Err(e) = render_engine.render_full_display(system_state).await {
        log::error!("Failed to render full display: {:?}", e);
        return;
    }

    // 刷新到屏幕（全屏刷新）
    if let Err(e) = render_engine.flush_full().await {
        log::error!("Failed to perform full refresh: {:?}", e);
    }
}

/// 处理局部刷新
async fn handle_partial_refresh(
    render_engine: &mut RenderEngine,
    refresh_type: &PartialRefreshType,
) {
    log::debug!("Performing partial refresh: {:?}", refresh_type);

    match refresh_type {
        PartialRefreshType::TimeOnly => {
            if let Err(e) = render_engine.render_time(SYSTEM_STATE.lock().await).await {
                log::error!("Failed to render time: {:?}", e);
                return;
            }
        }

        PartialRefreshType::DateOnly => {
            if let Err(e) = render_engine.render_date(SYSTEM_STATE).await {
                log::error!("Failed to render date: {:?}", e);
                return;
            }
        }

        PartialRefreshType::WeatherOnly => {
            if let Err(e) = render_engine.render_weather(SYSTEM_STATE).await {
                log::error!("Failed to render weather: {:?}", e);
                return;
            }
        }

        PartialRefreshType::QuoteOnly => {
            if let Err(e) = render_engine.render_quote(SYSTEM_STATE).await {
                log::error!("Failed to render quote: {:?}", e);
                return;
            }
        }

        PartialRefreshType::StatusOnly => {
            if let Err(e) = render_engine.render_status(SYSTEM_STATE).await {
                log::error!("Failed to render status: {:?}", e);
                return;
            }
        }

        PartialRefreshType::TimeAndDate => {
            if let Err(e) = render_engine.render_time(SYSTEM_STATE).await {
                log::error!("Failed to render time: {:?}", e);
                return;
            }
            if let Err(e) = render_engine.render_date(SYSTEM_STATE).await {
                log::error!("Failed to render date: {:?}", e);
                return;
            }
        }
    }

    // 刷新到屏幕（局部刷新）
    if let Err(e) = render_engine.flush_partial().await {
        log::error!("Failed to perform partial refresh: {:?}", e);
    }
}

/// 处理组件更新
async fn handle_component_update(
    render_engine: &mut RenderEngine,
    SYSTEM_STATE: &SystemState,
    component_type: ComponentType,
    component_data: ComponentData,
    last_full_refresh: &mut Option<Instant>,
) {
    match (component_type, component_data) {
        (ComponentType::Time, ComponentData::TimeData(time_data)) => {
            log::debug!("Updating time component");

            // 更新时间
            SYSTEM_STATE.lock().await.update_time(time_data.clone());

            // 渲染时间组件
            if let Err(e) = render_engine.render_time(SYSTEM_STATE).await {
                log::error!("Failed to render time: {:?}", e);
                return;
            }

            // 触发日期更新（每分钟）
            SYSTEM_STATE.lock().await.request_date_update();

            // 检查是否需要全屏刷新（每15分钟）
            check_and_trigger_full_refresh(*last_full_refresh).await;

            // 局部刷新时间
            if let Err(e) = render_engine.flush_partial().await {
                log::error!("Failed to update time: {:?}", e);
            }
        }

        (ComponentType::Date, ComponentData::DateData(date_data)) => {
            log::debug!("Updating date component");

            // 更新日期（包含农历）
            SYSTEM_STATE.lock().await.update_date(date_data.clone());

            // 渲染日期组件
            if let Err(e) = render_engine.render_date(SYSTEM_STATE).await {
                log::error!("Failed to render date: {:?}", e);
                return;
            }

            // 局部刷新日期
            if let Err(e) = render_engine.flush_partial().await {
                log::error!("Failed to update date: {:?}", e);
            }
        }

        (ComponentType::Weather, ComponentData::WeatherData(weather_data)) => {
            log::debug!("Updating weather component");

            // 更新天气
            SYSTEM_STATE
                .lock()
                .await
                .update_weather(weather_data.clone());

            // 渲染天气组件
            if let Err(e) = render_engine.render_weather(SYSTEM_STATE).await {
                log::error!("Failed to render weather: {:?}", e);
                return;
            }

            // 局部刷新天气
            if let Err(e) = render_engine.flush_partial().await {
                log::error!("Failed to update weather: {:?}", e);
            }
        }

        (ComponentType::Quote, ComponentData::QuoteData(quote_data)) => {
            log::debug!("Updating quote component");

            // 更新格言
            SYSTEM_STATE.lock().await.update_quote(quote_data.clone());

            // 渲染格言组件
            if let Err(e) = render_engine.render_quote(SYSTEM_STATE).await {
                log::error!("Failed to render quote: {:?}", e);
                return;
            }

            // 局部刷新格言
            if let Err(e) = render_engine.flush_partial().await {
                log::error!("Failed to update quote: {:?}", e);
            }
        }

        (ComponentType::Battery, ComponentData::BatteryData(battery_data)) => {
            log::debug!("Updating battery component");

            // 更新电池状态
            SYSTEM_STATE
                .lock()
                .await
                .update_battery_status(battery_data.clone());

            // 渲染状态组件
            if let Err(e) = render_engine.render_status(SYSTEM_STATE).await {
                log::error!("Failed to render status: {:?}", e);
                return;
            }

            // 局部刷新状态
            if let Err(e) = render_engine.flush_partial().await {
                log::error!("Failed to update battery status: {:?}", e);
            }
        }

        (ComponentType::Network, ComponentData::NetworkData(network_data)) => {
            log::debug!("Updating network component");

            // 更新网络状态
            SYSTEM_STATE
                .lock()
                .await
                .update_network_status(network_data.clone());

            // 渲染状态组件
            if let Err(e) = render_engine.render_status(SYSTEM_STATE).await {
                log::error!("Failed to render status: {:?}", e);
                return;
            }

            // 局部刷新状态
            if let Err(e) = render_engine.flush_partial().await {
                log::error!("Failed to update network status: {:?}", e);
            }
        }

        _ => {
            log::warn!("Unhandled component update: {:?}", component_type);
        }
    }
}

/// 处理农历计算请求
async fn handle_lunar_request(SYSTEM_STATE: &SharedSystemState) {
    log::debug!("Received lunar calculation request");

    // 这里可以触发异步农历计算
    // 为了不阻塞显示线程，我们使用 embassy_futures::spawn
    let state_clone = SYSTEM_STATE.clone();

    embassy_futures::spawn(async move {
        // 获取当前时间
        if let Some(time_data) = state_clone.lock().await.current_time.clone() {
            // 异步计算农历
            // let lunar_data = calculate_lunar(&time_data).await;

            // 发送更新事件
            // let date_data = DateData {
            //     solar: time_data,
            //     lunar: Some(lunar_data),
            // };

            // DISPLAY_EVENTS.send(DisplayEvent::UpdateComponent(
            //     ComponentType::Date,
            //     ComponentData::DateData(date_data),
            // )).await;
        }
    });
}

/// 检查并触发全屏刷新（每15分钟）
async fn check_and_trigger_full_refresh(last_full_refresh: Option<Instant>) {
    if let Some(last_refresh) = last_full_refresh {
        if Instant::now() - last_refresh >= Duration::from_secs(15 * 60) {
            log::info!("15 minutes elapsed, triggering full refresh");
            DISPLAY_EVENTS.send(DisplayEvent::FullRefresh).await;
        }
    }
}
