use embassy_executor::Spawner;

use crate::common::error::Result;
use crate::kernel::driver::button::DefaultButtonDriver;
use crate::kernel::driver::buzzer::DefaultBuzzerDriver;
use crate::kernel::driver::display::DefaultDisplayDriver;
use crate::kernel::driver::led::DefaultLedDriver;
use crate::kernel::driver::network::DefaultNetworkDriver;
use crate::kernel::driver::power::DefaultPowerDriver;
use crate::kernel::driver::sensor::DefaultSensorDriver;
use crate::kernel::driver::storage::DefaultConfigStorageDriver;
use crate::kernel::driver::time_driver::DefaultTimeDriver;

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
