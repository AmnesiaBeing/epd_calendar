// src/tasks/status_task.rs
use embassy_time::{Duration, Ticker};

use crate::driver::network::{DefaultNetworkDriver, NetworkDriver};
use crate::driver::power::{DefaultPowerMonitor, PowerMonitor};
use crate::tasks::{ComponentData, ComponentType, DISPLAY_EVENTS, DisplayEvent};

#[embassy_executor::task]
pub async fn run(power_monitor: DefaultPowerMonitor, network_driver: DefaultNetworkDriver) {
    let mut ticker = Ticker::every(Duration::from_secs(1 * 60)); // 每1分钟检查一次状态

    let mut last_battery = None;
    let mut last_charging = None;
    let mut last_network = None;

    loop {
        ticker.next().await;

        // 检查电池状态变化
        let battery = power_monitor.battery_level().await;
        if last_battery != Some(battery) {
            last_battery = Some(battery);
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(
                    ComponentType::Battery,
                    ComponentData::BatteryData(battery),
                ))
                .await;
        }

        // 检查充电状态变化
        let charging = power_monitor.is_charging().await;
        if last_charging != Some(charging) {
            last_charging = Some(charging);
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(
                    ComponentType::Battery,
                    ComponentData::ChargingStatus(charging),
                ))
                .await;
        }

        // 检查网络状态变化
        let network = network_driver.is_connected();
        if last_network != Some(network) {
            last_network = Some(network);
            DISPLAY_EVENTS
                .send(DisplayEvent::UpdateComponent(
                    ComponentType::Network,
                    ComponentData::NetworkStatus(network),
                ))
                .await;
        }
    }
}
