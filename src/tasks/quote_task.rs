// src/tasks/quote_task.rs
use embassy_time::{Duration, Ticker};

use crate::service::QuoteService;
use crate::tasks::{ComponentData, DISPLAY_EVENTS, DisplayEvent};

#[embassy_executor::task]
pub async fn quote_task(quote_service: QuoteService) {
    let mut ticker = Ticker::every(Duration::from_secs(1 * 60 * 60)); // 每2小时

    loop {
        ticker.next().await;

        if let Ok(quote) = quote_service.get_random_quote().await {
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(ComponentData::QuoteData(
                    quote,
                )))
                .await;
        }
    }
}
