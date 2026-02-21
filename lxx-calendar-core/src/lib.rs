#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use embassy_executor::Spawner;

mod managers;
mod platform;
mod services;

use crate::{
    managers::{EventProducer, StateManager},
    services::{
        audio_service::AudioService, ble_service::BLEService, display_service::DisplayService,
        network_sync_service::NetworkSyncService, power_service::PowerManager,
        quote_service::QuoteService, time_service::TimeService,
    },
};
use lxx_calendar_common::*;

static EVENT_CHANNEL: LxxSystemEventChannel = LxxSystemEventChannel::new()

pub async fn main_task<P: PlatformTrait>(
    spawner: Spawner,
    platform_ctx: PlatformContext<P>,
) -> SystemResult<()> {
    info!("lxx-calendar starting...");

    let event_channel = Box::new(LxxSystemEventChannel::new());
    let event_channel_static = Box::leak(event_channel);
    let event_receiver = event_channel_static.receiver();
    let event_sender = event_channel_static.sender();

    let mut time_service = TimeService::new(&platform_ctx.rtc);
    let mut display_service = DisplayService::new();
    let mut quote_service = QuoteService::new();
    let mut ble_service = BLEService::new();
    let mut power_manager = PowerManager::new();
    let mut audio_service = AudioService::new(platform_ctx.audio);
    let mut network_service = NetworkSyncService::new(&mut platform_ctx.rtc);

    time_service.initialize().await?;
    display_service.initialize().await?;
    quote_service.initialize().await?;
    ble_service.initialize().await?;
    power_manager.initialize().await?;
    audio_service.initialize().await?;
    network_service.initialize().await?;

    info!("All services initialized");

    let mut event_producer = EventProducer::new();
    event_producer.initialize().await;

    let mut config_manager = managers::ConfigManager::with_event_sender(event_sender);
    config_manager.initialize().await?;
    let config = config_manager.load_config().await?;
    info!(
        "Configuration loaded, hour_chime_enabled: {}",
        config.time_config.hour_chime_enabled
    );

    let mut state_manager: StateManager<P> = StateManager::new(
        event_receiver,
        &mut time_service,
        &mut display_service,
        &mut quote_service,
        &mut ble_service,
        &mut power_manager,
        &mut audio_service,
        &mut network_service,
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
