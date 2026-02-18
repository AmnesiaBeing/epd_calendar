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
        network_service::NetworkService, power_service::PowerManager, time_service::TimeService,
    },
};
use lxx_calendar_common::*;

pub async fn main_task<P: PlatformTrait>(
    _spawner: Spawner,
    _platform_ctx: PlatformContext<P>,
) -> SystemResult<()> {
    info!("lxx-calendar starting...");

    let event_channel = Box::new(LxxSystemEventChannel::new());
    let event_channel_static = Box::leak(event_channel);
    let event_receiver = event_channel_static.receiver();
    let event_sender = event_channel_static.sender();

    let mut time_service = TimeService::new();
    let mut display_service = DisplayService::new();
    let mut network_service = NetworkService::new();
    let mut ble_service = BLEService::new();
    let mut power_manager = PowerManager::new();
    let mut audio_service = AudioService::new();

    time_service.initialize().await?;
    display_service.initialize().await?;
    network_service.initialize().await?;
    ble_service.initialize().await?;
    power_manager.initialize().await?;
    audio_service.initialize().await?;

    info!("All services initialized");

    let mut event_producer = EventProducer::new();
    event_producer.initialize().await;

    let sender = event_sender;
    let _producer = async move {
        EventProducer::start_minute_timer(sender).await;
    };

    let mut state_manager = StateManager::new(
        event_receiver,
        &mut time_service,
        &mut display_service,
        &mut network_service,
        &mut ble_service,
        &mut power_manager,
        &mut audio_service,
    );

    state_manager.initialize().await?;
    state_manager.start().await?;

    let wakeup_event = SystemEvent::WakeupEvent(WakeupEvent::WakeFromDeepSleep);
    state_manager.handle_event(wakeup_event).await?;

    info!("Main task started, entering event loop");

    loop {
        match state_manager.wait_for_event().await {
            Ok(event) => {
                debug!("Received event: {:?}", event);
                if let Err(e) = state_manager.handle_event(event).await {
                    error!("Failed to handle event: {:?}", e);
                }
            }
            Err(e) => {
                error!("Failed to wait for event: {:?}", e);
            }
        }
    }
}
