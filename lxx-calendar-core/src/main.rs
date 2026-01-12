#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use embassy_executor::Spawner;

mod managers;
mod platform;
mod services;

use crate::{managers::StateManager, platform::*};
use lxx_calendar_common::*;

#[platform_main]
async fn main(spawner: Spawner) {
    main_task(spawner).await.unwrap();
}

async fn main_task(spawner: Spawner) -> SystemResult<()> {
    let _ = Platform::init(spawner);

    info!("lxx-calendar starting...");

    // 创建系统事件通道
    let channel = Box::new(LxxChannel::<SystemEvent>::new());
    let channel_static = Box::leak(channel);
    let receiver = channel_static.receiver();

    let mut state_manager = StateManager::new(receiver);
    state_manager.initialize().await?;
    state_manager.start().await?;

    info!("Main task started");

    loop {
        match state_manager.wait_for_event().await {
            Ok(event) => {
                info!("Received event: {:?}", event);
                if let Err(e) = handle_event(&mut state_manager, event).await {
                    error!("Failed to handle event: {:?}", e);
                }
            }
            Err(e) => {
                error!("Failed to wait for event: {:?}", e);
            }
        }
    }
}

async fn handle_event(state_manager: &mut StateManager, event: SystemEvent) -> SystemResult<()> {
    match state_manager.handle_event(event).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to handle event: {:?}", e);
            Err(e)
        }
    }
}
