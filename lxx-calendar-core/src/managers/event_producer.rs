use embassy_executor::{Spawner, task};
use embassy_time::{Duration, Timer};

use lxx_calendar_common::{LxxChannelSender, SystemEvent, *};

pub struct EventProducer {
    initialized: bool,
}

impl EventProducer {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub async fn initialize(&mut self) {
        info!("Initializing event producer");
        self.initialized = true;
    }

    pub fn start_ble_timeout_timer(
        &self,
        spawner: Spawner,
        sender: LxxChannelSender<'static, SystemEvent>,
    ) {
        spawner.spawn(ble_timeout_task(sender)).ok();
    }
}

#[task]
async fn ble_timeout_task(sender: LxxChannelSender<'static, SystemEvent>) {
    info!("Starting BLE timeout timer (5 minutes)");
    Timer::after(Duration::from_secs(300)).await;
    let event = SystemEvent::SystemStateEvent(SystemStateEvent::EnterDeepSleep);
    let _ = sender.send(event).await;
}
