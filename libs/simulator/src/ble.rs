use crate::rtc::SleepState;
use lxx_calendar_common::traits::ble::BLEDriver;
use lxx_calendar_common::types::ConfigChange;
use lxx_calendar_common::info;
use std::sync::{Arc, Mutex};

pub struct SimulatedBLE {
    connected: bool,
    advertising: bool,
    configured: bool,
    connected_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
    disconnected_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
    data_callback: Arc<Mutex<Option<Box<dyn Fn(&[u8]) + Send + 'static>>>>,
    sleep_state: Option<SleepState>,
}

impl SimulatedBLE {
    pub fn new() -> Self {
        Self {
            connected: false,
            advertising: false,
            configured: false,
            connected_callback: Arc::new(Mutex::new(None)),
            disconnected_callback: Arc::new(Mutex::new(None)),
            data_callback: Arc::new(Mutex::new(None)),
            sleep_state: None,
        }
    }

    pub fn set_external_wakeup_flag(
        &mut self,
        _sleep_flag: Arc<Mutex<bool>>,
    ) {
        // 不需要额外存储，BLE 唤醒通过 request_wakeup 直接操作标志
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
        // 唤醒系统
        if let Some(ref sleep_state) = self.sleep_state {
            sleep_state.request_wakeup();
        }
        info!("Simulated BLE connected");
        if let Ok(guard) = self.connected_callback.lock() {
            if let Some(ref cb) = *guard {
                cb();
            }
        }
    }

    pub fn simulate_disconnect(&mut self) {
        self.connected = false;
        info!("Simulated BLE disconnected");
        if let Ok(guard) = self.disconnected_callback.lock() {
            if let Some(ref cb) = *guard {
                cb();
            }
        }
    }

    pub fn simulate_config(&mut self, data: &[u8]) -> ConfigChange {
        self.configured = true;

        // 唤醒系统
        if let Some(ref sleep_state) = self.sleep_state {
            sleep_state.request_wakeup();
        }

        // 解析 JSON 数据以确定配置类型
        let change = if let Ok(json_str) = std::str::from_utf8(data) {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                // 根据 type 字段判断配置类型
                let config_type = json_value
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                info!("Simulated BLE config type: {}", config_type);

                match config_type {
                    "wifi_config" | "network_config" => ConfigChange::NetworkConfig,
                    "display_config" => ConfigChange::DisplayConfig,
                    "time_config" => ConfigChange::TimeConfig,
                    "power_config" => ConfigChange::PowerConfig,
                    "log_config" => ConfigChange::LogConfig,
                    "command" => {
                        // 命令类型不改变配置状态
                        ConfigChange::NetworkConfig
                    }
                    _ => {
                        // 未知类型，根据数据长度回退到旧逻辑
                        if data.len() < 10 {
                            ConfigChange::TimeConfig
                        } else if data.len() < 50 {
                            ConfigChange::NetworkConfig
                        } else if data.len() < 100 {
                            ConfigChange::DisplayConfig
                        } else if data.len() < 150 {
                            ConfigChange::PowerConfig
                        } else {
                            ConfigChange::LogConfig
                        }
                    }
                }
            } else {
                // JSON 解析失败，使用旧逻辑
                Self::guess_config_type_by_length(data.len())
            }
        } else {
            // UTF-8 解码失败，使用旧逻辑
            Self::guess_config_type_by_length(data.len())
        };

        info!("Simulated BLE config applied: {:?}", change);

        if let Ok(guard) = self.data_callback.lock() {
            if let Some(ref cb) = *guard {
                cb(data);
            }
        }

        change
    }

    /// 根据数据长度猜测配置类型（回退逻辑）
    fn guess_config_type_by_length(len: usize) -> ConfigChange {
        if len < 10 {
            ConfigChange::TimeConfig
        } else if len < 50 {
            ConfigChange::NetworkConfig
        } else if len < 100 {
            ConfigChange::DisplayConfig
        } else if len < 150 {
            ConfigChange::PowerConfig
        } else {
            ConfigChange::LogConfig
        }
    }

    pub fn simulate_advertising(&mut self) {
        self.advertising = true;
        info!("Simulated BLE advertising");
    }
}

impl Default for SimulatedBLE {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SimulatedBLE {
    fn clone(&self) -> Self {
        Self {
            connected: self.connected,
            advertising: self.advertising,
            configured: self.configured,
            connected_callback: Arc::clone(&self.connected_callback),
            disconnected_callback: Arc::clone(&self.disconnected_callback),
            data_callback: Arc::clone(&self.data_callback),
            sleep_state: self.sleep_state.clone(),
        }
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

    async fn start_advertising(&mut self) -> Result<(), Self::Error> {
        self.advertising = true;
        info!("Simulated BLE start advertising");
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Self::Error> {
        self.advertising = false;
        self.connected = false;
        info!("Simulated BLE stop");
        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        info!("Simulated BLE initialized");
        Ok(())
    }

    async fn set_connected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        if let Ok(mut guard) = self.connected_callback.lock() {
            *guard = Some(callback);
        }
    }

    async fn set_disconnected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        if let Ok(mut guard) = self.disconnected_callback.lock() {
            *guard = Some(callback);
        }
    }

    async fn set_data_callback(&mut self, callback: Box<dyn Fn(&[u8]) + Send + 'static>) {
        if let Ok(mut guard) = self.data_callback.lock() {
            *guard = Some(callback);
        }
    }

    async fn notify(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        info!("Simulated BLE notify: {} bytes", data.len());
        Ok(())
    }
}
