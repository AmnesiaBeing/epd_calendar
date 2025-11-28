// src/tasks/weather_task.rs
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::app_core::display_manager::DisplayManager;
use crate::common::error::{AppError, Result};
use crate::common::types::DisplayData;
use crate::service::weather_service::WeatherService;

#[embassy_executor::task]
pub async fn weather_task(
    display_manager: &'static Mutex<ThreadModeRawMutex, DisplayManager>,
    display_data: &'static Mutex<ThreadModeRawMutex, DisplayData>,
    weather_service: &'static Mutex<ThreadModeRawMutex, WeatherService>,
) {
    log::debug!("Weather task started");

    let mut last_successful_update = Instant::now();
    let mut consecutive_failures = 0;
    const MAX_CONSECUTIVE_FAILURES: u8 = 5;
    const NORMAL_UPDATE_INTERVAL: Duration = Duration::from_secs(2 * 60 * 60); // 2小时
    const RETRY_INTERVAL: Duration = Duration::from_secs(10 * 60); // 10分钟

    loop {
        let time_since_last_update = Instant::now() - last_successful_update;
        let should_update =
            time_since_last_update > NORMAL_UPDATE_INTERVAL || consecutive_failures > 0;

        if should_update {
            match weather_service.lock().await.get_weather().await {
                Ok(weather) => {
                    log::debug!("Weather data updated: {:?}", weather);

                    {
                        let mut data = display_data.lock().await;
                        data.weather = weather;
                    }

                    // 标记天气区域需要刷新
                    if let Err(e) = display_manager.lock().await.mark_dirty("weather") {
                        log::warn!("Failed to mark weather region dirty: {}", e);
                    }

                    last_successful_update = Instant::now();
                    consecutive_failures = 0;

                    // 成功更新后使用正常间隔
                    Timer::after(NORMAL_UPDATE_INTERVAL).await;
                }
                Err(e) => {
                    consecutive_failures += 1;
                    log::warn!(
                        "Weather update failed (attempt {}): {}",
                        consecutive_failures,
                        e
                    );

                    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        log::error!(
                            "Too many consecutive weather update failures, giving up for now"
                        );
                        consecutive_failures = 0; // 重置，稍后再试
                        Timer::after(NORMAL_UPDATE_INTERVAL).await;
                    } else {
                        // 失败后使用重试间隔
                        Timer::after(RETRY_INTERVAL).await;
                    }
                }
            }
        } else {
            // 等待下一次更新检查
            Timer::after(Duration::from_secs(60)).await; // 每分钟检查一次
        }
    }
}
