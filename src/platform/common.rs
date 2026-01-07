use crate::common::error::Result;
pub trait Platform: Sized {
    type Peripherals;

    fn init() -> Result<Self>;

    fn peripherals(&self) -> &Self::Peripherals;

    fn peripherals_mut(&mut self) -> &mut Self::Peripherals;

    fn init_logging(&self);

    fn init_rtos(&mut self);
}

#[cfg(feature = "esp32")]
pub type DefaultPlatform = super::esp32::Esp32Platform;

#[cfg(feature = "tspi")]
pub type DefaultPlatform = super::tspi::TspiPlatform;

#[cfg(feature = "simulator")]
pub type DefaultPlatform = super::simulator::SimulatorPlatform;
