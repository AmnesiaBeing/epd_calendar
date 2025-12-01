// src/tasks/status_task.rs
use embassy_time::{Duration, Timer};

use crate::common::types::{DisplayData, GlobalMutex, StatusData};
use crate::driver::network::DefaultNetworkDriver;
use crate::driver::power::{DefaultPowerMonitor, PowerMonitor};

#[embassy_executor::task]
pub async fn status_task(
    display_data: &'static GlobalMutex<DisplayData<'static>>,
    power_monitor: &'static GlobalMutex<DefaultPowerMonitor>,
    network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
) {
    log::debug!("Status task started");

    let mut last_status = StatusData::default();

    loop {
        // 获取当前系统状态
        let current_status = StatusData {
            is_charging: power_monitor.lock().await.is_charging().await,
            battery_level: power_monitor.lock().await.battery_level().await,
            is_online: network_driver.lock().await.is_connected().await,
        };

        // 检查状态是否变化
        if current_status != last_status {
            log::debug!(
                "Status changed: charging={}, battery={:?}, online={}",
                current_status.is_charging,
                current_status.battery_level,
                current_status.is_online
            );

            {
                let mut data = display_data.lock().await;
                data.status = current_status.clone();
            }

            // 标记状态区域需要刷新
            // if let Err(e) = display_manager.lock().await.mark_dirty("status") {
            //     log::warn!("Failed to mark status region dirty: {}", e);
            // }

            last_status = current_status;
        }

        // 每30秒检查一次状态
        Timer::after(Duration::from_secs(30)).await;
    }
}
