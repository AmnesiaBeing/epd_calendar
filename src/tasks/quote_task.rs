// src/tasks/quote_task.rs
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};

use crate::app_core::display_manager::DisplayManager;
use crate::common::types::DisplayData;
use crate::service::quote_service::QuoteService;

#[embassy_executor::task]
pub async fn quote_task(
    display_manager: &'static Mutex<NoopRawMutex, DisplayManager>,
    display_data: &'static Mutex<NoopRawMutex, DisplayData<'static>>,
    quote_service: &'static QuoteService,
) {
    log::debug!("Quote task started");

    // 初始延迟，让其他更重要的任务先运行
    Timer::after(Duration::from_secs(1)).await;

    loop {
        match quote_service.get_random_quote().await {
            Ok(new_hitokoto) => {
                log::debug!(
                    "Selected Random Quote: {}",
                    &new_hitokoto.hitokoto[..new_hitokoto.hitokoto.len().min(20)]
                );

                {
                    let mut data = display_data.lock().await;
                    data.quote = new_hitokoto;
                }

                // 标记格言区域需要刷新
                if let Err(e) = display_manager.lock().await.mark_dirty("quote") {
                    log::warn!("Failed to mark quote region dirty: {}", e);
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
