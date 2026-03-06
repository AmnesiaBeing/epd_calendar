//! Simulator Configuration Manager

use lxx_calendar_common::types::config::{
    SystemConfig, NetworkConfig, TimeConfig, DisplayConfig, PowerConfig, LogConfig,
    DisplayTheme, LogMode, LogLevel, EncryptedString,
};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Simulator Configuration
pub struct SimulatorConfig {
    config: Arc<RwLock<SystemConfig>>,
}

impl SimulatorConfig {
    pub fn new() -> Arc<Self> {
        let config = SystemConfig {
            version: 1,
            time_config: TimeConfig {
                timezone_offset: 8,
                alarms: heapless::Vec::new(),
                hour_chime_enabled: false,
                auto_sleep_start: Some((23, 0)),
                auto_sleep_end: Some((7, 0)),
            },
            network_config: NetworkConfig {
                wifi_ssid: heapless::String::new(),
                wifi_password: EncryptedString { 
                    data: heapless::Vec::new(), 
                    iv: heapless::Vec::new() 
                },
                weather_api_key: EncryptedString { 
                    data: heapless::Vec::new(), 
                    iv: heapless::Vec::new() 
                },
                location_id: heapless::String::new(),
                sync_interval_minutes: 30,
            },
            display_config: DisplayConfig {
                theme: DisplayTheme::Default,
                low_power_refresh_enabled: true,
                refresh_interval_seconds: 60,
            },
            power_config: PowerConfig {
                low_battery_threshold: 20,
                critical_battery_threshold: 5,
                low_power_mode_enabled: false,
            },
            log_config: LogConfig {
                log_mode: LogMode::Log,
                log_level: LogLevel::Info,
                log_to_flash: false,
            },
        };

        Arc::new(Self {
            config: Arc::new(RwLock::new(config)),
        })
    }

    pub async fn get_config(&self) -> SystemConfig {
        self.config.read().await.clone()
    }

    pub async fn get_network_config(&self) -> NetworkConfig {
        self.config.read().await.network_config.clone()
    }

    pub async fn set_network_config(&self, config: NetworkConfig) {
        let mut c = self.config.write().await;
        c.network_config = config;
    }

    pub async fn get_time_config(&self) -> TimeConfig {
        self.config.read().await.time_config.clone()
    }

    pub async fn set_time_config(&self, config: TimeConfig) {
        let mut c = self.config.write().await;
        c.time_config = config;
    }

    pub async fn get_display_config(&self) -> DisplayConfig {
        self.config.read().await.display_config.clone()
    }

    pub async fn set_display_config(&self, config: DisplayConfig) {
        let mut c = self.config.write().await;
        c.display_config = config;
    }

    pub async fn get_power_config(&self) -> PowerConfig {
        self.config.read().await.power_config.clone()
    }

    pub async fn set_power_config(&self, config: PowerConfig) {
        let mut c = self.config.write().await;
        c.power_config = config;
    }

    pub async fn get_log_config(&self) -> LogConfig {
        self.config.read().await.log_config.clone()
    }

    pub async fn set_log_config(&self, config: LogConfig) {
        let mut c = self.config.write().await;
        c.log_config = config;
    }
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self::new()
    }
}
