// src/tasks/time_task.rs
use embassy_time::{Duration, Ticker};

use crate::{
    service::TimeService,
    tasks::{ComponentData, DISPLAY_EVENTS, DisplayEvent},
};

#[embassy_executor::task]
pub async fn time_task(time_service: TimeService) {
    let mut ticker = Ticker::every(Duration::from_secs(60));

    loop {
        ticker.next().await;

        // 获取当前时间
        let time_data = time_service.get_current_time().unwrap();

        // 发送时间更新事件
        DISPLAY_EVENTS
            .send(DisplayEvent::UpdateComponent(ComponentData::TimeData(
                time_data.clone(),
            )))
            .await;

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
