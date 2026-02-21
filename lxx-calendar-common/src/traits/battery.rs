use crate::*;

pub trait Battery {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn read_voltage(&mut self) -> Result<u16, Self::Error>;

    async fn is_low_battery(&mut self) -> Result<bool, Self::Error>;

    async fn is_charging(&mut self) -> Result<bool, Self::Error>;

    fn enable_voltage_interrupt<F>(
        &mut self,
        threshold_mv: u16,
        callback: F,
    ) -> Result<(), Self::Error>
    where
        F: Fn() + Send + 'static;

    fn enable_charging_interrupt<F>(&mut self, callback: F) -> Result<(), Self::Error>
    where
        F: Fn() + Send + 'static;
}

pub struct NoBattery {
    default_voltage: u16,
    is_low: bool,
    is_charging: bool,
}

impl NoBattery {
    pub fn new(default_voltage: u16, is_low: bool, is_charging: bool) -> Self {
        Self {
            default_voltage,
            is_low,
            is_charging,
        }
    }
}

impl Battery for NoBattery {
    type Error = core::convert::Infallible;

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        info!(
            "[NoBattery] initialized: voltage={}mV, low={}, charging={}",
            self.default_voltage, self.is_low, self.is_charging
        );
        Ok(())
    }

    async fn read_voltage(&mut self) -> Result<u16, Self::Error> {
        Ok(self.default_voltage)
    }

    async fn is_low_battery(&mut self) -> Result<bool, Self::Error> {
        Ok(self.is_low)
    }

    async fn is_charging(&mut self) -> Result<bool, Self::Error> {
        Ok(self.is_charging)
    }

    fn enable_voltage_interrupt<F>(
        &mut self,
        _threshold_mv: u16,
        _callback: F,
    ) -> Result<(), Self::Error>
    where
        F: Fn() + Send + 'static,
    {
        Ok(())
    }

    fn enable_charging_interrupt<F>(&mut self, _callback: F) -> Result<(), Self::Error>
    where
        F: Fn() + Send + 'static,
    {
        Ok(())
    }
}
