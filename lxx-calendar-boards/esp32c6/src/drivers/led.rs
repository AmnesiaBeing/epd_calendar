use esp_hal::{
    gpio::AnyPin,
    ledc::{self, Ledc, LowSpeed, channel},
    peripherals::Peripherals,
};
use lxx_calendar_common::{LEDDriver, LEDIndicatorState};

const BREATHE_FREQ: u32 = 1;
const BLINK_FAST: u32 = 200;
const BLINK_NORMAL: u32 = 1000;

pub struct Esp32LED {
    pin: AnyPin<'static>,
}

impl Esp32LED {
    pub fn new(peripherals: &Peripherals, spawner: embassy_executor::Spawner) -> Self {
        Self {
            pin: unsafe { peripherals.GPIO9.clone_unchecked() }.into(),
        }
    }
}

impl LEDDriver for Esp32LED {
    type Error = core::convert::Infallible;

    fn set_state(&mut self, state: LEDIndicatorState) -> Result<(), Self::Error> {
        todo!()
    }
}
