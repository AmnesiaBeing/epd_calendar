#![no_std]

extern crate alloc;

use embassy_executor::Spawner;
use lxx_calendar_common::*;

use crate::{
    managers::StateManager,
    services::{
        audio_service::AudioService, ble_service::BLEService,
        network_sync_service::NetworkSyncService, power_service::PowerManager,
        quote_service::QuoteService, time_service::TimeService,
    },
};

mod managers;
mod platform;
mod services;

pub async fn main_task<P: PlatformTrait>(
    _spawner: Spawner,
    event_receiver: LxxChannelReceiver<'static, SystemEvent>,
    event_sender: LxxChannelSender<'static, SystemEvent>,
    platform_ctx: PlatformContext<P>,
) -> SystemResult<()> {
    info!("lxx-calendar starting...");

    let time_service = TimeService::new().with_rtc(platform_ctx.rtc);
    let quote_service = QuoteService::new();
    let ble_service = BLEService::new();
    let mut power_manager = PowerManager::<P::BatteryDevice>::new(event_sender);
    power_manager.set_battery_device(platform_ctx.battery);
    let audio_service = AudioService::new(platform_ctx.audio);
    let network_sync_service = NetworkSyncService::new();

    let mut config_manager = managers::ConfigManager::with_event_sender(event_sender);
    config_manager.initialize().await?;
    let config = config_manager.load_config().await?;
    info!(
        "Configuration loaded, hour_chime_enabled: {}",
        config.time_config.hour_chime_enabled
    );

    let mut state_manager: StateManager<P> = StateManager::new(
        event_receiver,
        event_sender,
        time_service,
        quote_service,
        ble_service,
        power_manager,
        audio_service,
        network_sync_service,
        platform_ctx.sys_watch_dog,
    );
    state_manager.with_config(&config);

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
