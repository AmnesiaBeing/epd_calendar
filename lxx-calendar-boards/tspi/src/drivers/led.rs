use lxx_calendar_common::*;
use lxx_calendar_common::{LEDDriver, LEDIndicatorState};

pub struct TspiLED;

impl LEDDriver for TspiLED {
    type Error = core::convert::Infallible;

    fn set_state(&mut self, state: LEDIndicatorState) -> Result<(), Self::Error> {
        info!("[Tspi LED] State: {:?}", state);
        Ok(())
    }
}
