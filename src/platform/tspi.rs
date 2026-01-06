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
use crate::platform::common::Platform;

pub struct TspiPlatform;

impl Platform for TspiPlatform {
    type Peripherals = ();

    fn init() -> Result<Self> {
        Ok(Self)
    }

    fn peripherals(&mut self) -> &mut Self::Peripherals {
        &mut ()
    }

    async fn create_display_driver(&mut self) -> Result<DefaultDisplayDriver> {
        DefaultDisplayDriver::new().await
    }

    async fn create_network_driver(&mut self, spawner: &Spawner) -> Result<DefaultNetworkDriver> {
        let mut driver = DefaultNetworkDriver::new();
        driver.initialize(spawner).await?;
        Ok(driver)
    }

    fn create_buzzer_driver(&mut self) -> Result<DefaultBuzzerDriver> {
        DefaultBuzzerDriver::new()
    }

    fn create_time_driver(&mut self) -> Result<DefaultTimeDriver> {
        DefaultTimeDriver::new()
    }

    fn create_storage_driver(&mut self) -> Result<DefaultConfigStorageDriver> {
        DefaultConfigStorageDriver::new("flash.bin", 4096)
    }

    fn create_power_driver(&mut self) -> Result<DefaultPowerDriver> {
        DefaultPowerDriver::new()
    }

    fn create_sensor_driver(&mut self) -> Result<DefaultSensorDriver> {
        DefaultSensorDriver::new()
    }

    fn create_led_driver(&mut self, _spawner: &Spawner) -> Result<DefaultLedDriver> {
        DefaultLedDriver::new()
    }

    fn create_button_driver(&mut self) -> Result<DefaultButtonDriver> {
        DefaultButtonDriver::new()
    }

    fn init_logging() {
        env_logger::init();
        log::info!("Initialized env_logger for Tspi");
    }

    #[cfg(feature = "esp32")]
    fn init_rtos(&mut self) {}
}
