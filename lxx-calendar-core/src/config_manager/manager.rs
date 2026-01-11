use lxx_calendar_common as lxx_common;
use lxx_common::{SystemResult, SystemError};

use alloc::string::ToString;

pub struct ConfigManager {
    initialized: bool,
    config: Option<lxx_common::SystemConfig>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            initialized: false,
            config: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), lxx_common::SystemError> {
        lxx_common::info!("Initializing config manager");
        self.initialized = true;
        Ok(())
    }

    pub async fn load_config(&mut self) -> Result<lxx_common::SystemConfig, lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        lxx_common::info!("Loading config");
        Ok(lxx_common::SystemConfig {
            version: 1,
            time_config: lxx_common::TimeConfig {
                timezone_offset: 28800,
                alarms: heapless::Vec::new(),
                hour_chime_enabled: true,
                auto_sleep_start: None,
                auto_sleep_end: None,
            },
            network_config: lxx_common::NetworkConfig {
                wifi_ssid: heapless::String::new(),
                wifi_password: lxx_common::EncryptedString {
                    data: heapless::Vec::new(),
                    iv: heapless::Vec::new(),
                },
                weather_api_key: lxx_common::EncryptedString {
                    data: heapless::Vec::new(),
                    iv: heapless::Vec::new(),
                },
                location_id: heapless::String::new(),
                sync_interval_minutes: 120,
            },
            display_config: lxx_common::DisplayConfig {
                theme: lxx_common::DisplayTheme::Default,
                low_power_refresh_enabled: true,
                refresh_interval_seconds: 60,
            },
            power_config: lxx_common::PowerConfig {
                low_battery_threshold: 30,
                critical_battery_threshold: 10,
                low_power_mode_enabled: true,
            },
            log_config: lxx_common::LogConfig {
                log_mode: lxx_common::LogMode::Defmt,
                log_level: lxx_common::LogLevel::Info,
                log_to_flash: true,
            },
        })
    }

    pub async fn save_config(&mut self, config: lxx_common::SystemConfig) -> Result<(), lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        lxx_common::info!("Saving config");
        self.config = Some(config);
        Ok(())
    }

    pub async fn get_config(&self) -> Result<lxx_common::SystemConfig, lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        self.config.clone().ok_or_else(|| {
            lxx_common::SystemError::StorageError(lxx_common::StorageError::NotFound)
        })
    }
}