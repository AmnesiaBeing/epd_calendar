#![no_std]
#![allow(unused_imports)]

extern crate alloc;

use static_cell::StaticCell;

use lxx_calendar_common::{
    compiled_config, debug, error,
    events::SystemEvent,
    info,
    storage::{ConfigPersistence, FlashDevice},
    traits::{LxxSystemEventChannel, NetworkStack, PlatformContext, PlatformTrait},
    types::{SystemConfig, SystemMode, SystemResult},
    warn,
};
use crate::{
    managers::StateManager,
    services::{
        audio_service::AudioService, ble_service::BLEService, button_service::ButtonService,
        network_sync_service::NetworkSyncService, power_service::PowerManager,
        quote_service::QuoteService, time_service::TimeService,
    },
};

mod managers;
mod services;

static EVENT_CHANNEL: StaticCell<LxxSystemEventChannel> = StaticCell::new();

pub async fn main_task<P: PlatformTrait>(
    _spawner: embassy_executor::Spawner,
    platform_ctx: PlatformContext<P>,
) -> SystemResult<()> {
    info!("lxx-calendar starting...");

    // 初始化静态事件通道
    let event_channel = EVENT_CHANNEL.init(LxxSystemEventChannel::new());
    let event_sender = event_channel.sender();
    let event_receiver = event_channel.receiver();

    let time_service = TimeService::new().with_rtc(platform_ctx.rtc);
    let quote_service = QuoteService::new();
    let ble_service = BLEService::new(platform_ctx.ble);
    let mut power_manager = PowerManager::<P::BatteryDevice>::new(event_sender);
    power_manager.set_battery_device(platform_ctx.battery);
    let audio_service = AudioService::new(platform_ctx.audio);

    let mut network_sync_service = NetworkSyncService::new();
    if let Some(stack) = platform_ctx.network.get_stack() {
        network_sync_service.set_stack(*stack);
    }

    // 设置编译期配置的 Open-Meteo 位置
    network_sync_service.set_location(
        compiled_config::openmeteo_latitude(),
        compiled_config::openmeteo_longitude(),
        compiled_config::openmeteo_location_name(),
    );

    let mut button_service = ButtonService::<P::ButtonDevice>::new(event_sender);
    button_service.set_button_device(platform_ctx.button);

    let config_persistence = ConfigPersistence::new(platform_ctx.flash);
    let config_manager =
        managers::ConfigManager::with_event_sender(config_persistence, event_sender.clone());

    let mut state_manager: StateManager<P, P::FlashDevice> = StateManager::new(
        event_receiver,
        event_sender,
        button_service,
        time_service,
        quote_service,
        ble_service,
        power_manager,
        audio_service,
        network_sync_service,
        platform_ctx.wifi,
        platform_ctx.sys_watch_dog,
        config_manager,
    );

    state_manager.initialize().await?;

    state_manager.transition_to(SystemMode::NormalWork).await?;

    info!("Main task started, entering event loop");

    state_manager.feed_watchdog();

    loop {
        match state_manager.wait_for_event().await {
            Ok(event) => {
                debug!("Received event: {:?}", event);
                state_manager.feed_watchdog();
                if let Err(e) = state_manager.handle_event(event).await {
                    error!("Failed to handle event: {:?}", e);
                }
                if let Err(e) = state_manager.schedule_next_wakeup().await {
                    error!("Failed to schedule next wakeup: {:?}", e);
                }
            }
            Err(e) => {
                error!("Failed to wait for event: {:?}", e);
            }
        }
    }
}
