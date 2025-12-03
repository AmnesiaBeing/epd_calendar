// src/tasks/quote_task.rs
use embassy_time::{Duration, Ticker};

use crate::service::QuoteService;
use crate::tasks::{ComponentData, DISPLAY_EVENTS, DisplayEvent};

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
                    .send(DisplayEvent::UpdateComponent(ComponentData::QuoteData(
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
