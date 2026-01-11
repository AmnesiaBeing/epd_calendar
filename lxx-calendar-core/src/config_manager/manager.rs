use lxx_calendar_common as lxxcc;
use lxxcc::{SystemResult, SystemError};

use alloc::string::ToString;

pub struct ConfigManager {
    initialized: bool,
    config: Option<lxxcc::SystemConfig>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            initialized: false,
            config: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), lxxcc::SystemError> {
        lxxcc::info!("Initializing config manager");
        self.initialized = true;
        Ok(())
    }

    pub async fn load_config(&mut self) -> Result<lxxcc::SystemConfig, lxxcc::SystemError> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Loading config");
        Ok(lxxcc::SystemConfig {
            version: 1,
            time_config: lxxcc::TimeConfig {
                timezone_offset: 28800,
                alarms: heapless::Vec::new(),
                hour_chime_enabled: true,
                auto_sleep_start: None,
                auto_sleep_end: None,
            },
            network_config: lxxcc::NetworkConfig {
                wifi_ssid: heapless::String::new(),
                wifi_password: lxxcc::EncryptedString {
                    data: heapless::Vec::new(),
                    iv: heapless::Vec::new(),
                },
                weather_api_key: lxxcc::EncryptedString {
                    data: heapless::Vec::new(),
                    iv: heapless::Vec::new(),
                },
                location_id: heapless::String::new(),
                sync_interval_minutes: 120,
            },
            display_config: lxxcc::DisplayConfig {
                theme: lxxcc::DisplayTheme::Default,
                low_power_refresh_enabled: true,
                refresh_interval_seconds: 60,
            },
            power_config: lxxcc::PowerConfig {
                low_battery_threshold: 30,
                critical_battery_threshold: 10,
                low_power_mode_enabled: true,
            },
            log_config: lxxcc::LogConfig {
                log_mode: lxxcc::LogMode::Defmt,
                log_level: lxxcc::LogLevel::Info,
                log_to_flash: true,
            },
        })
    }

    pub async fn save_config(&mut self, config: lxxcc::SystemConfig) -> Result<(), lxxcc::SystemError> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Saving config");
        self.config = Some(config);
        Ok(())
    }

    pub async fn get_config(&self) -> Result<lxxcc::SystemConfig, lxxcc::SystemError> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        self.config.clone().ok_or_else(|| {
            lxxcc::SystemError::StorageError(lxxcc::StorageError::NotFound)
        })
    }
}