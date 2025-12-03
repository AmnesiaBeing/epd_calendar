// src/tasks/status_task.rs
use embassy_time::{Duration, Ticker};

use crate::common::{ChargingStatus, GlobalMutex, NetworkStatus};
use crate::driver::network::{DefaultNetworkDriver, NetworkDriver};
use crate::driver::power::{DefaultPowerMonitor, PowerMonitor};
use crate::tasks::{ComponentData, DISPLAY_EVENTS, DisplayEvent};

#[embassy_executor::task]
pub async fn status_task(
    power_monitor: &'static GlobalMutex<DefaultPowerMonitor>,
    network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
) {
    let mut ticker = Ticker::every(Duration::from_secs(1 * 60)); // 每1分钟检查一次状态

    let mut last_battery = None;
    let mut last_charging = None;
    let mut last_network = None;

    loop {
        ticker.next().await;

        // 检查电池状态变化
        let battery = power_monitor.lock().await.battery_level().await;
        if last_battery != Some(battery) {
            last_battery = Some(battery);
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(ComponentData::BatteryData(
                    battery,
                )))
                .await;
        }

        // 检查充电状态变化
        let charging = power_monitor.lock().await.is_charging().await;
        if last_charging != Some(charging) {
            last_charging = Some(charging);
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(
                    ComponentData::ChargingStatus(ChargingStatus(charging)),
                ))
                .await;
        }

        // 检查网络状态变化
        let network = network_driver.lock().await.is_connected();
        if last_network != Some(network) {
            last_network = Some(network);
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(ComponentData::NetworkStatus(
                    NetworkStatus(network),
                )))
                .await;
        }
    }
}
