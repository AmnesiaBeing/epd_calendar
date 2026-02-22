
use lxx_calendar_common::ButtonDriver;
use lxx_calendar_common::{ButtonEvent, info, SystemEvent, UserEvent};

pub struct ButtonTask {
    event_sender: embassy_sync::channel::Sender<
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        SystemEvent,
        10,
    >,
}

impl ButtonTask {
    pub fn new(
        event_sender: embassy_sync::channel::Sender<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            SystemEvent,
            10,
        >,
    ) -> Self {
        Self { event_sender }
    }

    pub async fn run<P: ButtonDriver>(self, mut button: P) -> core::convert::Infallible {
        info!("Button task started");

        loop {
            match button.wait_for_press(embassy_time::Duration::from_secs(1)).await {
                Ok(event) => {
                    info!("Button event received: {:?}", event);
                    let system_event = match event {
                        ButtonEvent::DoubleClick => {
                            info!("Double click detected - No function yet");
                            SystemEvent::UserEvent(UserEvent::ButtonDoubleClick)
                        }
                        ButtonEvent::TripleClick => {
                            info!("Triple click detected - Entering pairing mode");
                            SystemEvent::UserEvent(UserEvent::ButtonTripleClick)
                        }
                        ButtonEvent::ShortPress => {
                            info!("Short press detected - Waking device");
                            SystemEvent::WakeupEvent(lxx_calendar_common::events::WakeupEvent::WakeByButton)
                        }
                        ButtonEvent::LongPress => {
                            info!("Long press detected (>15s) - Restoring factory defaults");
                            SystemEvent::UserEvent(UserEvent::ButtonLongPress)
                        }
                    };

                    self.event_sender.send(system_event).await;
                }
                Err(e) => {
                    info!("Failed to wait for button press");
                }
            }
        }
    }
}
