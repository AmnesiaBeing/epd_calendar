use crate::types::ConfigChange;

pub trait BLEDriver: Send {
    type Error;

    fn is_connected(&self) -> Result<bool, Self::Error>;
    fn is_advertising(&self) -> Result<bool, Self::Error>;
    fn is_configured(&self) -> Result<bool, Self::Error>;

    fn start_advertising(&mut self) -> Result<(), Self::Error>;
    fn stop(&mut self) -> Result<(), Self::Error>;
}

pub struct NoBLE;

impl NoBLE {
    pub fn new() -> Self {
        Self
    }
}

impl BLEDriver for NoBLE {
    type Error = core::convert::Infallible;

    fn is_connected(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn is_advertising(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn is_configured(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn start_advertising(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
