// src/tasks/time_task.rs

//! 时间任务模块 - 定时获取和更新系统时间
//! 
//! 该模块定时从时间服务获取当前时间，并发送更新事件到显示任务。

use embassy_time::{Duration, Ticker};

use crate::{
    common::error::{AppError, Result},
    service::TimeService,
    tasks::{ComponentDataType, DISPLAY_EVENTS, DisplayEvent},
};

// 配置常量
const TIME_UPDATE_INTERVAL_SECONDS: u64 = 60; // 每分钟更新一次显示时间

/// 时间任务主函数
#[embassy_executor::task]
pub async fn time_task(mut time_service: TimeService) {
    log::info!("🕒 Time task started");

    let mut ticker = Ticker::every(Duration::from_secs(TIME_UPDATE_INTERVAL_SECONDS));

    // 任务启动时立即更新一次时间
    if let Err(e) = update_time_display(&mut time_service).await {
        log::warn!("Initial time update failed: {:?}", e);
    }

    loop {
        ticker.next().await;

        // 更新显示时间
        let _ = update_time_display(&mut time_service).await;

        // 记录调试信息
        log::debug!("Time task tick - Display updated");
    }
}

/// 更新显示时间
/// 
/// # 参数
/// - `time_service`: 时间服务实例
/// 
/// # 返回值
/// - `Result<()>`: 更新成功返回Ok(()), 失败返回错误
async fn update_time_display(time_service: &mut TimeService) -> Result<()> {
    log::debug!("Updating time display");

    match time_service.get_current_time().await {
        Ok(time_data) => {
            log::debug!("Got time data: {:?}", time_data);

            // 发送时间更新事件到显示任务
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(ComponentDataType::TimeType(
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