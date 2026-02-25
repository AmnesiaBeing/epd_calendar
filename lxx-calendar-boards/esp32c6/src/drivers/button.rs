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
    pub fn new() -> Self {
        Self
    }

    pub fn init(&self, _peripherals: &Peripherals) {
    }

    pub fn start_monitoring(&self, spawner: Spawner, peripherals: Peripherals) {
        spawner.spawn(button_monitor_task(peripherals)).unwrap();
    }
}

impl Default for Esp32Button {
    fn default() -> Self {
        Self
    }
}

#[embassy_executor::task]
async fn button_monitor_task(peripherals: Peripherals) {
    let button = Input::new(
        unsafe { peripherals.GPIO0.clone_unchecked() },
        esp_hal::gpio::InputConfig::default().with_pull(Pull::Up),
    );
    
    let mut last_state = button.is_high();
    let mut press_start: Option<embassy_time::Instant> = None;

    loop {
        let current_state = button.is_low();

        if current_state && !last_state {
            press_start = Some(embassy_time::Instant::now());
        } else if !current_state && last_state {
            press_start = None;
        }

        if current_state {
            if let Some(start) = press_start {
                let duration = embassy_time::Instant::now().duration_since(start);
                if duration.as_millis() >= LONG_PRESS_MIN_MS as u64 {
                    press_start = None;
                    let signal = BUTTON_SIGNAL.init(Signal::new());
                    signal.signal(ButtonEvent::LongPress);
                }
            }
        }

        last_state = current_state;
        Timer::after_millis(DEBOUNCE_MS as u64).await;
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
