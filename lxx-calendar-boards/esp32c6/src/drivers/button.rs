use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use esp_hal::gpio::{Input, Pull};
use esp_hal::peripherals::Peripherals;
use lxx_calendar_common::traits::button::{
    ButtonDriver, ButtonEvent, DEBOUNCE_MS, LONG_PRESS_MIN_MS,
};
use lxx_calendar_common::*;
use static_cell::StaticCell;

static BUTTON_SIGNAL: StaticCell<Signal<CriticalSectionRawMutex, ButtonEvent>> =
    StaticCell::new();

pub struct Esp32Button;

impl Esp32Button {
    pub fn new(spawner: embassy_executor::Spawner) -> Self {
        spawner.spawn(esp32_button_task()).ok();
        Self
    }
}

impl ButtonDriver for Esp32Button {
    type Error = core::convert::Infallible;

    async fn register_press_callback<F>(&mut self, _callback: F) -> Result<(), Self::Error>
    where
        F: Fn(ButtonEvent) + Send + 'static,
    {
        let signal = BUTTON_SIGNAL.init(Signal::new());
        
        loop {
            let event = signal.wait().await;
            info!("Button event: {:?}", event);
        }
    }
}
