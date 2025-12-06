// src/tasks/status_task.rs

//! 状态任务模块 - 监控系统状态变化
//!
//! 该模块定时检查电池电量、充电状态和网络连接状态，并在状态变化时发送更新事件。

use embassy_time::{Duration, Ticker};

use crate::common::{BatteryLevel, ChargingStatus, GlobalMutex, NetworkStatus};
use crate::driver::network::{DefaultNetworkDriver, NetworkDriver};
use crate::driver::power::{DefaultPowerDriver, PowerDriver};
use crate::tasks::{ComponentDataType, DISPLAY_EVENTS, DisplayEvent};

/// 状态任务主函数
#[embassy_executor::task]
pub async fn status_task(
    power_driver: DefaultPowerDriver,
    network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
) {
    log::info!("Status task started");

    let mut ticker = Ticker::every(Duration::from_secs(1 * 60)); // 每1分钟检查一次状态

    let mut last_battery = None;
    let mut last_charging = None;
    let mut last_network = None;

    loop {
        ticker.next().await;
        log::debug!("Checking system status");

        // 检查电池状态变化
        let battery = power_driver.battery_level().await;
        if let Ok(battery) = battery {
            if last_battery != Some(battery) {
                log::info!("Battery level changed: {:?}%", battery);
                last_battery = Some(battery);
                DISPLAY_EVENTS
                    .send(DisplayEvent::UpdateComponent(
                        ComponentDataType::BatteryType(BatteryLevel::from_percent(battery)),
                    ))
                    .await;
            }
        }

        // 检查充电状态变化
        let charging = power_driver.is_charging().await;
        if let Ok(charging) = charging {
            if last_charging != Some(charging) {
                log::info!("Charging status changed: {}", charging);
                last_charging = Some(charging);
                DISPLAY_EVENTS
                    .send(DisplayEvent::UpdateComponent(
                        ComponentDataType::ChargingStatusType(ChargingStatus(charging)),
                    ))
                    .await;
            }
        }

        // 检查网络状态变化
        let network = network_driver.lock().await.is_connected();
        if last_network != Some(network) {
            log::info!("Network status changed: {}", network);
            last_network = Some(network);
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(
                    ComponentDataType::NetworkStatusType(NetworkStatus(network)),
                ))
                .await;
        }
    }
}
