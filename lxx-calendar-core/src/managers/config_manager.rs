use lxx_calendar_common as lxx_common;
use lxx_calendar_common::SystemEvent;

pub struct ConfigManager {
    initialized: bool,
    config: Option<lxx_common::SystemConfig>,
    event_sender: Option<lxx_common::LxxChannelSender<'static, SystemEvent>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            initialized: false,
            config: None,
            event_sender: None,
        }
    }

    pub fn with_event_sender(sender: lxx_common::LxxChannelSender<'static, SystemEvent>) -> Self {
        Self {
            initialized: false,
            config: None,
            event_sender: Some(sender),
        }
    }

    pub async fn initialize(&mut self) -> Result<(), lxx_common::SystemError> {
        lxx_common::info!("Initializing config manager");
        self.initialized = true;
        Ok(())
    }

    pub async fn set_event_sender(
        &mut self,
        sender: lxx_common::LxxChannelSender<'static, SystemEvent>,
    ) {
        self.event_sender = Some(sender);
    }

    pub async fn load_config(
        &mut self,
    ) -> Result<lxx_common::SystemConfig, lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
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

    pub async fn save_config(
        &mut self,
        config: lxx_common::SystemConfig,
    ) -> Result<(), lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }
        lxx_common::info!("Saving config");
        self.config = Some(config.clone());
        self.notify_config_changed(config).await;
        Ok(())
    }

    async fn notify_config_changed(&self, _config: lxx_common::SystemConfig) {
        if let Some(ref sender) = self.event_sender {
            for change in [
                lxx_calendar_common::ConfigChange::TimeConfig,
                lxx_calendar_common::ConfigChange::NetworkConfig,
                lxx_calendar_common::ConfigChange::DisplayConfig,
                lxx_calendar_common::ConfigChange::PowerConfig,
                lxx_calendar_common::ConfigChange::LogConfig,
            ] {
                let event = lxx_calendar_common::SystemEvent::ConfigChanged(change);
                let _ = sender.send(event).await;
            }
        }
    }

    pub async fn get_config(&self) -> Result<lxx_common::SystemConfig, lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }
        self.config.clone().ok_or_else(|| {
            lxx_common::SystemError::StorageError(lxx_common::StorageError::NotFound)
        })
    }
}
