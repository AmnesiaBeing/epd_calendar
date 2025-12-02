// src/tasks/weather_task.rs
use embassy_time::{Duration, Ticker};

use crate::service::WeatherService;
use crate::tasks::{ComponentData, ComponentType, DISPLAY_EVENTS, DisplayEvent};

#[embassy_executor::task]
pub async fn run(mut weather_service: WeatherService) {
    let mut ticker = Ticker::every(Duration::from_secs(2 * 60 * 60)); // 每2小时

    loop {
        ticker.next().await;

        // 获取天气数据
        if let Ok(weather_data) = weather_service.get_weather().await {
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(
                    ComponentType::Weather,
                    ComponentData::WeatherData(weather_data),
                ))
                .await;
        }
    }
}
