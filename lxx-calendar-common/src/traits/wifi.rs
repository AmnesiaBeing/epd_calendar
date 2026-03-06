#![allow(async_fn_in_trait)]

use crate::{HardwareError, SystemError, SystemResult};
use serde::{Deserialize, Serialize};

pub trait WifiController: Send + Sync {
    type Error;

    async fn connect_sta(&mut self, ssid: &str, password: &str) -> Result<(), Self::Error>;

    async fn disconnect(&mut self) -> Result<(), Self::Error>;

    fn is_connected(&self) -> bool;
    
    /// 测试 WiFi 连接（DNS + HTTP）
    async fn test_connection(&self) -> Result<WifiTestResult, Self::Error>;
    
    /// 扫描附近的 WiFi
    async fn scan(&self) -> Result<heapless::Vec<WifiInfo, 10>, Self::Error>;
}

/// WiFi 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiInfo {
    pub ssid: String,
    pub rssi: i16,
    pub is_encrypted: bool,
}

/// WiFi 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiTestResult {
    pub success: bool,
    pub ip: Option<String>,
    pub dns_resolved: bool,
    pub http_test: bool,
    pub message: String,
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
    pub fn new_sta(ssid: &str, password: &str) -> SystemResult<Self> {
        Ok(Self {
            ssid: heapless::String::try_from(ssid)
                .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))?,
            password: heapless::String::try_from(password)
                .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))?,
            mode: WifiMode::Sta,
        })
    }
}

pub struct NoWifi;

impl NoWifi {
    pub fn new() -> Self {
        Self
    }
}

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
    
    async fn test_connection(&self) -> Result<WifiTestResult, Self::Error> {
        Ok(WifiTestResult {
            success: false,
            ip: None,
            dns_resolved: false,
            http_test: false,
            message: "WiFi not available".to_string(),
        })
    }
    
    async fn scan(&self) -> Result<heapless::Vec<WifiInfo, 10>, Self::Error> {
        Ok(heapless::Vec::new())
    }
}
