use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LEDIndicatorState {
    Off,
    BlinkFast,
    BlinkSlow,
    On,
}

pub trait LEDDriver {
    type Error;
    fn set_state(&mut self, state: LEDIndicatorState) -> Result<(), Self::Error>;
}

pub struct NoLED;

impl NoLED {
    pub fn new() -> Self {
        Self
    }
}

impl LEDDriver for NoLED {
    type Error = core::convert::Infallible;

    fn set_state(&mut self, state: LEDIndicatorState) -> Result<(), Self::Error> {
        info!("[No LED] State: {:?}", state);
        Ok(())
    }
}
