// src/tasks/time_task.rs
use embassy_time::{Duration, Instant, Ticker};

use crate::{
    common::error::{AppError, Result},
    service::TimeService,
    tasks::{ComponentData, DISPLAY_EVENTS, DisplayEvent},
};

// 配置常量
const TIME_UPDATE_INTERVAL_SECONDS: u64 = 60; // 每分钟更新一次显示时间
const SNTP_UPDATE_INTERVAL_SECONDS: u64 = 6 * 60 * 60; // 每6小时同步一次网络时间
const MAX_SNTP_RETRY_ATTEMPTS: u8 = 3; // SNTP最大重试次数
const SNTP_RETRY_DELAY_SECONDS: u64 = 30; // SNTP重试延迟

#[embassy_executor::task]
pub async fn time_task(mut time_service: TimeService) {
    log::info!("🕒 Time task started");

    let mut ticker = Ticker::every(Duration::from_secs(TIME_UPDATE_INTERVAL_SECONDS));
    let mut last_sntp_update = Instant::now();

    // 任务启动时立即更新一次时间
    if let Err(e) = update_time_display(&mut time_service).await {
        log::warn!("Initial time update failed: {:?}", e);
    }

    // 任务启动时尝试同步网络时间
    match perform_sntp_sync(&mut time_service).await {
        Ok(()) => {
            log::info!("Initial SNTP sync successful");
            last_sntp_update = Instant::now();
        }
        Err(e) => {
            log::warn!("Initial SNTP sync failed: {:?}", e);
        }
    }

    loop {
        ticker.next().await;

        // 1. 更新显示时间
        let _ = update_time_display(&mut time_service).await;

        // 2. 检查是否需要同步网络时间
        let time_since_last_sync = Instant::now() - last_sntp_update;

        if time_since_last_sync.as_secs() >= SNTP_UPDATE_INTERVAL_SECONDS {
            log::info!("Performing scheduled SNTP time sync");

            match perform_sntp_sync(&mut time_service).await {
                Ok(()) => {
                    log::info!("SNTP sync completed successfully");
                    last_sntp_update = Instant::now();
                }
                Err(e) => {
                    log::warn!("SNTP sync failed: {:?}", e);
                }
            }
        }

        // 3. 记录调试信息
        log::debug!(
            "Time task tick - Next SNTP sync in {} seconds",
            SNTP_UPDATE_INTERVAL_SECONDS.saturating_sub(time_since_last_sync.as_secs())
        );
    }
}

/// 更新显示时间
async fn update_time_display(time_service: &mut TimeService) -> Result<()> {
    log::debug!("Updating time display");

    match time_service.get_current_time() {
        Ok(time_data) => {
            log::debug!("Got time data: {:?}", time_data);

            // 发送时间更新事件到显示任务
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(ComponentData::TimeData(
                    time_data.clone(),
                )))
                .await;
            Ok(())
        }
        Err(e) => {
            // 时间获取失败时，这种情况不会发生的吧，打印个日志吧
            log::warn!("Failed to get current time: {:?}", e);
            Err(AppError::TimeError)
        }
    }
}

/// 执行SNTP时间同步（带有最大重试次数）
async fn perform_sntp_sync(time_service: &mut TimeService) -> Result<()> {
    for attempt in 1..=MAX_SNTP_RETRY_ATTEMPTS {
        log::info!("SNTP sync attempt {}/{}", attempt, MAX_SNTP_RETRY_ATTEMPTS);

        match time_service.update_time_by_sntp().await {
            Ok(()) => {
                log::info!("SNTP sync successful");

                return Ok(());
            }
            Err(e) => {
                log::warn!("SNTP sync attempt {} failed: {:?}", attempt, e);

                if attempt < MAX_SNTP_RETRY_ATTEMPTS {
                    log::info!("Waiting {} seconds before retry", SNTP_RETRY_DELAY_SECONDS);
                    embassy_time::Timer::after(Duration::from_secs(SNTP_RETRY_DELAY_SECONDS)).await;
                } else {
                    log::error!(
                        "SNTP sync failed after all {} attempts",
                        MAX_SNTP_RETRY_ATTEMPTS
                    );
                    return Err(e);
                }
            }
        }
    }

    // 理论上不会到达这里，因为循环会返回
    unreachable!()
}
