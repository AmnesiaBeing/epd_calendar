use lxx_calendar_common::traits::ble::BLEDriver;
use lxx_calendar_common::types::ConfigChange;

pub struct SimulatedBLE {
    connected: bool,
    advertising: bool,
    configured: bool,
}

impl SimulatedBLE {
    pub fn new() -> Self {
        Self {
            connected: false,
            advertising: false,
            configured: false,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn is_advertising(&self) -> bool {
        self.advertising
    }

    pub fn is_configured(&self) -> bool {
        self.configured
    }

    pub fn simulate_connect(&mut self) {
        self.connected = true;
        self.advertising = false;
        log::info!("Simulated BLE connected");
    }

    pub fn simulate_disconnect(&mut self) {
        self.connected = false;
        log::info!("Simulated BLE disconnected");
    }

    pub fn simulate_config(&mut self, data: &[u8]) -> ConfigChange {
        self.configured = true;

        let change = if data.len() < 10 {
            ConfigChange::TimeConfig
        } else if data.len() < 50 {
            ConfigChange::NetworkConfig
        } else if data.len() < 100 {
            ConfigChange::DisplayConfig
        } else if data.len() < 150 {
            ConfigChange::PowerConfig
        } else {
            ConfigChange::LogConfig
        };

        log::info!("Simulated BLE config applied: {:?}", change);
        change
    }

    pub fn simulate_advertising(&mut self) {
        self.advertising = true;
        log::info!("Simulated BLE advertising");
    }
}

impl Default for SimulatedBLE {
    fn default() -> Self {
        Self::new()
    }
}

impl BLEDriver for SimulatedBLE {
    type Error = core::convert::Infallible;

    fn is_connected(&self) -> Result<bool, Self::Error> {
        Ok(self.connected)
    }

    fn is_advertising(&self) -> Result<bool, Self::Error> {
        Ok(self.advertising)
    }

    fn is_configured(&self) -> Result<bool, Self::Error> {
        Ok(self.configured)
    }

    fn start_advertising(&mut self) -> Result<(), Self::Error> {
        self.advertising = true;
        log::info!("Simulated BLE start advertising");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        self.advertising = false;
        self.connected = false;
        log::info!("Simulated BLE stop");
        Ok(())
    }
}
