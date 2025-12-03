// src/tasks/time_task.rs
use embassy_time::{Duration, Ticker};

use crate::{
    service::TimeService,
    tasks::{ComponentData, DISPLAY_EVENTS, DisplayEvent},
};

#[embassy_executor::task]
pub async fn time_task(time_service: TimeService) {
    log::info!("Time task started");

    let mut ticker = Ticker::every(Duration::from_secs(60));

    loop {
        ticker.next().await;
        log::debug!("Updating time display");

        // 获取当前时间
        match time_service.get_current_time() {
            Ok(time_data) => {
                log::info!("Time updated successfully");
                // 发送时间更新事件
                DISPLAY_EVENTS
                    .send(DisplayEvent::UpdateComponent(ComponentData::TimeData(
                        time_data.clone(),
                    )))
                    .await;
            }
            Err(e) => {
                log::error!("Failed to get current time: {:?}", e);
            }
        }

        // 异步计算农历（不阻塞时间更新）
        // embassy_futures::spawn(update_lunar_date(time_data));
    }
}

// async fn update_lunar_date(time_data: TimeData) {
//     // 这里执行耗时的农历计算
//     let lunar_data = calculate_lunar(&time_data).await;

//     DISPLAY_EVENTS
//         .send(DisplayEvent::UpdateComponent(
//             ComponentType::Date,
//             ComponentData::DateData(DateData {
//                 solar: time_data,
//                 lunar: lunar_data,
//             }),
//         ))
//         .await;
// }
