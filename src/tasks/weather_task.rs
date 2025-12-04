// src/tasks/weather_task.rs
use embassy_time::{Duration, Ticker};

use crate::service::WeatherService;
use crate::tasks::{ComponentDataType, DISPLAY_EVENTS, DisplayEvent};

#[embassy_executor::task]
pub async fn weather_task(mut weather_service: WeatherService) {
    log::info!("Weather task started");

    let mut ticker = Ticker::every(Duration::from_secs(2 * 60 * 60)); // 每2小时

    loop {
        ticker.next().await;
        log::debug!("Fetching weather data");

        // 获取天气数据
        match weather_service.get_weather().await {
            Ok(weather_data) => {
                log::info!("Weather data retrieved successfully");
                DISPLAY_EVENTS
                    .send(DisplayEvent::UpdateComponent(
                        ComponentDataType::WeatherType(weather_data),
                    ))
                    .await;
            }
            Err(e) => {
                log::error!("Failed to fetch weather data: {:?}", e);
            }
        }
    }
}
