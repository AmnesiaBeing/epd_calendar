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

    pub async fn start_minute_timer(sender: LxxChannelSender<'static, SystemEvent>) {
        info!("Starting minute timer");
        loop {
            Timer::after(Duration::from_secs(60)).await;
            let event = SystemEvent::TimeEvent(TimeEvent::MinuteTick);
            let _ = sender.send(event).await;
        }
    }

    pub async fn start_hour_chime_timer(sender: LxxChannelSender<'static, SystemEvent>) {
        info!("Starting hour chime timer");
        loop {
            Timer::after(Duration::from_secs(3600)).await;
            let event = SystemEvent::TimeEvent(TimeEvent::HourChimeTrigger);
            let _ = sender.send(event).await;
        }
    }

    pub async fn start_ble_timeout(sender: LxxChannelSender<'static, SystemEvent>) {
        info!("Starting BLE timeout timer (5 minutes)");
        Timer::after(Duration::from_secs(300)).await;
        let event = SystemEvent::SystemStateEvent(SystemStateEvent::EnterDeepSleep);
        let _ = sender.send(event).await;
    }
}
