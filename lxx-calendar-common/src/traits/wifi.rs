#![allow(async_fn_in_trait)]

use crate::SystemError;

pub trait WifiController: Send + Sync {
    type Error;

    async fn connect_sta(&mut self, ssid: &str, password: &str) -> Result<(), Self::Error>;

    async fn disconnect(&mut self) -> Result<(), Self::Error>;

    fn is_connected(&self) -> bool;

    async fn get_rssi(&self) -> Result<i32, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiMode {
    Sta,
    Ap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiConfig {
    pub ssid: heapless::String<32>,
    pub password: heapless::String<64>,
    pub mode: WifiMode,
}

impl WifiConfig {
    pub fn new_sta(ssid: &str, password: &str) -> Result<Self, SystemError> {
        Ok(Self {
            ssid: heapless::String::try_from(ssid).map_err(|_| SystemError::InvalidParameter)?,
            password: heapless::String::try_from(password).map_err(|_| SystemError::InvalidParameter)?,
            mode: WifiMode::Sta,
        })
    }
}

pub struct NoWifi;

impl WifiController for NoWifi {
    type Error = core::convert::Infallible;

    async fn connect_sta(&mut self, _ssid: &str, _password: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        false
    }

    async fn get_rssi(&self) -> Result<i32, Self::Error> {
        Ok(0)
    }
}
