#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use embassy_executor::Spawner;

use lxx_calendar_common as lxx_common;
use lxx_common::{SystemEvent, SystemResult};
use lxx_common::types::async_types::{LxxChannel, LxxChannelReceiver};

mod audio_service;
mod ble_service;
mod config_manager;
mod display_service;
mod network_service;
mod power_manager;
mod state_manager;
mod time_service;

pub async fn core_main<M: lxx_common::types::async_types::LxxAsyncRawMutex + Default + 'static>(
    spawner: Spawner
) -> SystemResult<()> {
    // 注意：这里需要平台实现提供具体的platform实例
    // 目前暂时使用空实现，需要根据实际平台实现进行调整
    lxx_common::info!("lxx-calendar starting...");

    // 创建系统事件通道并确保其具有'static生命周期
    let channel = Box::new(LxxChannel::<M, SystemEvent, 32>::new());
    let channel_static = Box::leak(channel);
    let receiver = channel_static.receiver();
    
    let mut state_manager = state_manager::StateManager::new(receiver);
    state_manager.initialize().await?;
    state_manager.start().await?;

    lxx_common::info!("Main task started");

    loop {
        match state_manager.wait_for_event().await {
            Ok(event) => {
                lxx_common::info!("Received event: {:?}", event);
                if let Err(e) = handle_event(&mut state_manager, event).await {
                    lxx_common::error!("Failed to handle event: {:?}", e);
                }
            }
            Err(e) => {
                lxx_common::error!("Failed to wait for event: {:?}", e);
            }
        }
    }
}

async fn handle_event<M: lxx_common::types::async_types::LxxAsyncRawMutex>(
    state_manager: &mut state_manager::StateManager<M>,
    event: SystemEvent,
) -> SystemResult<()> {
    match state_manager.handle_event(event).await {
        Ok(_) => Ok(()),
        Err(e) => {
            lxx_common::error!("Failed to handle event: {:?}", e);
            Err(e)
        }
    }
}