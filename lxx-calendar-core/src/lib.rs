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
        power_service::PowerManager, quote_service::QuoteService, time_service::TimeService,
    },
};
use lxx_calendar_common::*;

pub async fn main_task<P: PlatformTrait>(
    spawner: Spawner,
    platform_ctx: PlatformContext<P>,
) -> SystemResult<()> {
    info!("lxx-calendar starting...");

    let event_channel = Box::new(LxxSystemEventChannel::new());
    let event_channel_static = Box::leak(event_channel);
    let event_receiver = event_channel_static.receiver();
    let event_sender = event_channel_static.sender();

    let mut time_service = TimeService::new(platform_ctx.rtc);
    let mut display_service = DisplayService::new();
    let mut quote_service = QuoteService::new();
    let mut ble_service = BLEService::new();
    let mut power_manager = PowerManager::new();
    let mut audio_service = AudioService::new(platform_ctx.audio);

    time_service.initialize().await?;
    display_service.initialize().await?;
    quote_service.initialize().await?;
    ble_service.initialize().await?;
    power_manager.initialize().await?;
    audio_service.initialize().await?;

    info!("All services initialized");

    let mut event_producer = EventProducer::new();
    event_producer.initialize().await;

    event_producer.start_minute_timer(spawner.clone(), event_sender);

    let mut state_manager: StateManager<'_, P> = StateManager::new(
        event_receiver,
        &mut time_service,
        &mut display_service,
        &mut quote_service,
        &mut ble_service,
        &mut power_manager,
        &mut audio_service,
        platform_ctx.sys_watch_dog,
    );

    state_manager.initialize().await?;
    state_manager.start().await?;

    state_manager.transition_to(SystemMode::NormalWork).await?;

    info!("Main task started, entering event loop");

    loop {
        state_manager.feed_watchdog();
        
        match state_manager.wait_for_event().await {
            Ok(event) => {
                debug!("Received event: {:?}", event);
                state_manager.feed_watchdog();
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
