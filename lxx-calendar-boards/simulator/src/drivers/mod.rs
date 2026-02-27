mod buzzer;
mod epd;
mod network;

pub use buzzer::SimulatorBuzzer;
pub use epd::init_epd;
pub use network::TunTapNetwork;

use lxx_calendar_common::traits::ble::BLEDriver;
use core::convert::Infallible;
use simulator::SimulatedBLE;

pub struct SimulatorBLE {
    inner: SimulatedBLE,
}

impl Default for SimulatorBLE {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatorBLE {
    pub fn new() -> Self {
        Self {
            inner: SimulatedBLE::new(),
        }
    }
}

impl BLEDriver for SimulatorBLE {
    type Error = Infallible;

    fn is_connected(&self) -> Result<bool, Self::Error> {
        Ok(self.inner.is_connected())
    }

    fn is_advertising(&self) -> Result<bool, Self::Error> {
        Ok(self.inner.is_advertising())
    }

    fn is_configured(&self) -> Result<bool, Self::Error> {
        Ok(self.inner.is_configured())
    }

    fn start_advertising(&mut self) -> Result<(), Self::Error> {
        self.inner.simulate_advertising();
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        self.inner.simulate_disconnect();
        Ok(())
    }
}
