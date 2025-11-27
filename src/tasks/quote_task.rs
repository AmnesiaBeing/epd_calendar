// src/tasks/quote_task.rs
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use log::debug;

use crate::app_core::display_manager::DisplayManager;
use crate::common::error::{AppError, Result};
use crate::common::types::DisplayData;
use crate::service::quote_service::QuoteService;

#[embassy_executor::task]
pub async fn quote_task(
    display_manager: Mutex<NoopRawMutex, DisplayManager>,
    display_data: Mutex<NoopRawMutex, DisplayData>,
    quote_service: QuoteService,
) {
    debug!("Quote task started");

    // 初始延迟，让其他更重要的任务先运行
    Timer::after(Duration::from_secs(30)).await;

    let mut last_quote = None;

    loop {
        match quote_service.get_random_quote().await {
            Ok(new_quote) => {
                // 只有当格言变化时才更新
                if Some(&new_quote) != last_quote.as_ref() {
                    debug!("Quote updated: {}", &new_quote[..new_quote.len().min(20)]);

                    {
                        let mut data = display_data.lock().await;
                        data.quote = new_quote.clone();
                    }

                    // 标记格言区域需要刷新
                    if let Err(e) = display_manager.lock().await.mark_dirty("quote") {
                        log::warn!("Failed to mark quote region dirty: {}", e);
                    }

                    last_quote = Some(new_quote);
                }
            }
            Err(e) => {
                log::warn!("Quote service error: {}", e);
            }
        }

        // 每24小时更新一次格言
        Timer::after(Duration::from_secs(24 * 60 * 60)).await;
    }
}
