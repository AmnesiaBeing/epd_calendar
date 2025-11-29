// src/tasks/time_task.rs
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};

use crate::{
    app_core::display_manager::DisplayManager,
    common::types::DisplayData,
    driver::storage::DefaultStorageDriver,
    service::{config_service::ConfigService, time_service::TimeService},
};

#[embassy_executor::task]
pub async fn time_task(
    display_manager: &'static Mutex<ThreadModeRawMutex, DisplayManager>,
    display_data: &'static Mutex<ThreadModeRawMutex, DisplayData<'static>>,
    time_service: &'static Mutex<ThreadModeRawMutex, TimeService>,
    config: &'static Mutex<ThreadModeRawMutex, ConfigService<DefaultStorageDriver>>,
) {
    log::info!("Time task initialized and started");
    let mut last_minute = None;
    let mut last_date = None;
    let mut time_service = time_service.lock().await;

    log::debug!("Time task entering main loop");
    loop {
        log::debug!("Time task iteration started");

        // 获取当前配置
        let current_config = config.lock().await.load_config().await.unwrap();
        log::debug!(
            "Time config loaded: 24h format={}, temperature unit={}",
            current_config.time_format_24h,
            if current_config.temperature_celsius {
                "Celsius"
            } else {
                "Fahrenheit"
            }
        );

        // 尝试通过SNTP同步时间
        log::debug!("Attempting SNTP time synchronization");
        if let Err(e) = time_service.update_time_by_sntp().await {
            log::warn!("Failed to update time by SNTP: {}", e);
        }

        // 应用配置到时间服务
        log::debug!("Applying time configuration to time service");
        time_service.set_24_hour_format(current_config.time_format_24h);
        time_service.set_temperature_celsius(current_config.temperature_celsius);

        // 获取当前时间
        match time_service.get_current_time().await {
            Ok(current_time) => {
                log::debug!(
                    "Successfully retrieved current time: {}",
                    current_time.date_string,
                );

                let current_minute = (current_time.hour, current_time.minute);
                let current_date = &current_time.date_string;

                // 检查分钟是否变化
                if Some(current_minute) != last_minute {
                    log::info!(
                        "Minute changed: {:02}:{:02}",
                        current_time.hour,
                        current_time.minute
                    );

                    {
                        let mut data = display_data.lock().await;
                        data.time = current_time.clone();
                    }

                    // 标记时间区域需要刷新
                    log::debug!("Marking time region as dirty for refresh");
                    if let Err(e) = display_manager.lock().await.mark_dirty("time") {
                        log::warn!("Failed to mark time region dirty: {}", e);
                    }

                    last_minute = Some(current_minute);
                }

                // 检查日期是否变化（跨天时）
                if Some(current_date) != last_date.as_ref() {
                    log::info!("Date changed: {}", current_date);

                    // 标记日期区域需要刷新
                    log::debug!("Marking date region as dirty for refresh");
                    if let Err(e) = display_manager.lock().await.mark_dirty("date") {
                        log::warn!("Failed to mark date region dirty: {}", e);
                    }

                    last_date = Some(current_date.clone());
                }
            }
            Err(e) => {
                log::warn!("Time service error: {}", e);
                // 短暂延迟后重试
                log::debug!("Retrying after 10 seconds due to time service error");
                Timer::after(Duration::from_secs(10)).await;
            }
        }

        // 每分钟更新一次时间
        log::debug!("Waiting for next time update (60 seconds)");
        Timer::after(Duration::from_secs(60)).await;
    }
}
