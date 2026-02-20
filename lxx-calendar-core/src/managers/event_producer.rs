use embassy_executor::{task, Spawner};
use embassy_time::{Duration, Timer};

use lxx_calendar_common::{LxxChannelSender, SystemEvent, TimeEvent, *};

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

    pub fn start_minute_timer(&self, spawner: Spawner, sender: LxxChannelSender<'static, SystemEvent>) {
        spawner.spawn(minute_timer_task(sender)).ok();
    }

    pub fn start_hour_chime_timer(&self, spawner: Spawner, sender: LxxChannelSender<'static, SystemEvent>) {
        spawner.spawn(hour_chime_timer_task(sender)).ok();
    }
}

#[task]
async fn minute_timer_task(sender: LxxChannelSender<'static, SystemEvent>) {
    info!("Starting minute timer");
    loop {
        Timer::after(Duration::from_secs(60)).await;
        let event = SystemEvent::TimeEvent(TimeEvent::MinuteTick);
        let _ = sender.send(event).await;
    }
}

#[task]
async fn hour_chime_timer_task(sender: LxxChannelSender<'static, SystemEvent>) {
    info!("Starting hour chime timer");
    loop {
        Timer::after(Duration::from_secs(3600)).await;
        let event = SystemEvent::TimeEvent(TimeEvent::HourChimeTrigger);
        let _ = sender.send(event).await;
    }
}

#[task]
async fn ble_timeout_task(sender: LxxChannelSender<'static, SystemEvent>) {
    info!("Starting BLE timeout timer (5 minutes)");
    Timer::after(Duration::from_secs(300)).await;
    let event = SystemEvent::SystemStateEvent(SystemStateEvent::EnterDeepSleep);
    let _ = sender.send(event).await;
}
