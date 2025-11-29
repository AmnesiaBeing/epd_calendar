// src/tasks/status_task.rs
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use log::debug;

use crate::app_core::display_manager::DisplayManager;
use crate::common::types::{DisplayData, StatusData};
use crate::driver::power::{DefaultPowerMonitor, PowerMonitor};

#[embassy_executor::task]
pub async fn status_task(
    display_manager: &'static Mutex<ThreadModeRawMutex, DisplayManager>,
    display_data: &'static Mutex<ThreadModeRawMutex, DisplayData<'static>>,
    power_monitor: &'static Mutex<ThreadModeRawMutex, DefaultPowerMonitor>,
    // network_driver: impl NetworkDriver,
) {
    debug!("Status task started");

    let mut last_status = StatusData::default();

    loop {
        // 获取当前系统状态
        let current_status = StatusData {
            is_charging: power_monitor.lock().await.is_charging().await,
            battery_level: power_monitor.lock().await.battery_level().await,
            // is_online: network_driver.is_connected().await,
        };

        // 检查状态是否变化
        if current_status != last_status {
            debug!(
                "Status changed: charging={}, battery={:?}",
                current_status.is_charging,
                current_status.battery_level,
                // current_status.is_online
            );

            {
                let mut data = display_data.lock().await;
                data.status = current_status.clone();
            }

            // 标记状态区域需要刷新
            if let Err(e) = display_manager.lock().await.mark_dirty("status") {
                log::warn!("Failed to mark status region dirty: {}", e);
            }

            last_status = current_status;
        }

        // 每30秒检查一次状态
        Timer::after(Duration::from_secs(30)).await;
    }
}
