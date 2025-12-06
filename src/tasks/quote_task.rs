// src/tasks/quote_task.rs

//! 名言任务模块 - 定时获取和更新名言数据
//! 
//! 该模块定时从名言服务获取随机名言，并发送更新事件到显示任务。

use embassy_time::{Duration, Ticker};

use crate::service::QuoteService;
use crate::tasks::{ComponentDataType, DISPLAY_EVENTS, DisplayEvent};

/// 名言任务主函数
#[embassy_executor::task]
pub async fn quote_task(quote_service: QuoteService) {
    log::info!("Quote task started");

    let mut ticker = Ticker::every(Duration::from_secs(1 * 60 * 60)); // 每2小时

    loop {
        ticker.next().await;
        log::debug!("Fetching new quote");

        match quote_service.get_random_quote().await {
            Ok(quote) => {
                log::info!("Quote retrieved successfully");
                DISPLAY_EVENTS
                    .send(DisplayEvent::UpdateComponent(ComponentDataType::QuoteType(
                        quote,
                    )))
                    .await;
            }
            Err(e) => {
                log::error!("Failed to fetch quote: {:?}", e);
            }
        }
    }
}