use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex};
use esp_hal::gpio::{Level, Output, OutputConfig};
use lxx_calendar_common::{LEDDriver, LEDIndicatorState};

const BLINK_FAST_MS: u64 = 200;
const BLINK_SLOW_MS: u64 = 1000;

type LedMutex = Mutex<CriticalSectionRawMutex, Option<Output<'static>>>;

static LED_MUTEX: LedMutex = Mutex::new(None);
static LED_COMMAND: Channel<CriticalSectionRawMutex, LEDIndicatorState, 1> = Channel::new();

pub struct Esp32LED<'d> {
    led_pin: Output<'d>,
    current_state: LEDIndicatorState,
}

impl<'d> Esp32LED<'d> {
    pub fn new(pin: impl esp_hal::gpio::OutputPin + 'd, spawner: &Spawner) -> Self {
        let led_pin = Output::new(pin, Level::Low, OutputConfig::default());

        let _ = spawner.spawn(led_blink_task());

        Self {
            led_pin,
            current_state: LEDIndicatorState::Off,
        }
    }

    pub async fn store_pin(&mut self) {
        let mut guard = LED_MUTEX.lock().await;
        let output = unsafe {
            core::ptr::read(&self.led_pin as *const Output<'d> as *const Output<'static>)
        };
        *guard = Some(output);
    }
}

impl<'d> LEDDriver for Esp32LED<'d> {
    type Error = core::convert::Infallible;

    fn set_state(&mut self, state: LEDIndicatorState) -> Result<(), Self::Error> {
        if self.current_state == state {
            return Ok(());
        }

        defmt::debug!(
            "LED state changing from {:?} to {:?}",
            self.current_state,
            state
        );
        self.current_state = state;

        let _ = LED_COMMAND.try_send(state);

        Ok(())
    }
}

#[embassy_executor::task]
async fn led_blink_task() {
    use embassy_futures::select::{Either, select};
    use embassy_time::{Duration, Timer};

    let mut current_state = LEDIndicatorState::Off;
    let receiver = LED_COMMAND.receiver();

    loop {
        match current_state {
            LEDIndicatorState::Off => {
                match select(receiver.receive(), core::future::pending::<()>()).await {
                    Either::First(new_state) => {
                        current_state = new_state;
                        defmt::debug!("LED task received: {:?}", current_state);
                    }
                    Either::Second(_) => unreachable!(),
                }
            }
            LEDIndicatorState::On => {
                match select(receiver.receive(), core::future::pending::<()>()).await {
                    Either::First(new_state) => {
                        current_state = new_state;
                        defmt::debug!("LED task received: {:?}", current_state);
                    }
                    Either::Second(_) => unreachable!(),
                }
            }
            LEDIndicatorState::BlinkFast => {
                match select(
                    receiver.receive(),
                    Timer::after(Duration::from_millis(BLINK_FAST_MS)),
                )
                .await
                {
                    Either::First(new_state) => {
                        current_state = new_state;
                        defmt::debug!("LED task received: {:?}", current_state);
                    }
                    Either::Second(_) => {
                        let mut guard = LED_MUTEX.lock().await;
                        if let Some(led) = guard.as_mut() {
                            led.toggle();
                        }
                    }
                }
            }
            LEDIndicatorState::BlinkSlow => {
                match select(
                    receiver.receive(),
                    Timer::after(Duration::from_millis(BLINK_SLOW_MS)),
                )
                .await
                {
                    Either::First(new_state) => {
                        current_state = new_state;
                        defmt::debug!("LED task received: {:?}", current_state);
                    }
                    Either::Second(_) => {
                        let mut guard = LED_MUTEX.lock().await;
                        if let Some(led) = guard.as_mut() {
                            led.toggle();
                        }
                    }
                }
            }
        }
    }
}
