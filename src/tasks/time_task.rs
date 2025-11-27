use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};

use crate::common::types::{DisplayData, SystemConfig};

// src/tasks/time_task.rs
#[embassy_executor::task]
pub async fn time_task(
    display_manager: Mutex<NoopRawMutex, DisplayManager>,
    display_data: Mutex<NoopRawMutex, DisplayData>,
    time_service: TimeService<impl TimeSource>,
    config: Mutex<NoopRawMutex, SystemConfig>,
) {
    log::debug!("Time task started");
    let mut last_minute = None;
    let mut last_date = None;

    loop {
        let current_config = config.lock().await.clone();

        // 应用配置到时间服务
        time_service.set_24_hour_format(current_config.time_format_24h);
        time_service.set_temperature_celsius(current_config.temperature_celsius);

        match time_service.get_current_time().await {
            Ok(current_time) => {
                let current_minute = (current_time.hour, current_time.minute);
                let current_date = &current_time.date_string;

                // 检查分钟是否变化
                if Some(current_minute) != last_minute {
                    debug!(
                        "Minute changed: {:02}:{:02}",
                        current_time.hour, current_time.minute
                    );

                    {
                        let mut data = display_data.lock().await;
                        data.time = current_time.clone();
                    }

                    // 标记时间区域需要刷新
                    if let Err(e) = display_manager.lock().await.mark_dirty("time") {
                        log::warn!("Failed to mark time region dirty: {}", e);
                    }

                    last_minute = Some(current_minute);
                }

                // 检查日期是否变化（跨天时）
                if Some(current_date) != last_date.as_ref() {
                    debug!("Date changed: {}", current_date);

                    // 标记日期区域需要刷新
                    if let Err(e) = display_manager.lock().await.mark_dirty("date") {
                        log::warn!("Failed to mark date region dirty: {}", e);
                    }

                    last_date = Some(current_date.clone());
                }
            }
            Err(e) => {
                log::warn!("Time service error: {}", e);
                // 短暂延迟后重试
                Timer::after(Duration::from_secs(10)).await;
            }
        }

        // 每分钟更新一次时间
        Timer::after(Duration::from_secs(60)).await;
    }
}
