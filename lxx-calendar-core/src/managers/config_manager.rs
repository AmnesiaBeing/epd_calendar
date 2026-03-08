use lxx_calendar_common as lxx_common;
use lxx_calendar_common::SystemEvent;
use lxx_calendar_common::storage::{ConfigPersistence, FlashDevice};
use lxx_calendar_common::types::config::ConfigChange;

/// 配置管理器
///
/// 负责配置的加载、保存和通知
pub struct ConfigManager<F: FlashDevice> {
    initialized: bool,
    config: Option<lxx_common::SystemConfig>,
    event_sender: Option<lxx_common::LxxChannelSender<'static, SystemEvent>>,
    persistence: ConfigPersistence<F>,
}

impl<F: FlashDevice> ConfigManager<F> {
    /// 创建新的配置管理器
    pub fn new(persistence: ConfigPersistence<F>) -> Self {
        Self {
            initialized: false,
            config: None,
            event_sender: None,
            persistence,
        }
    }

    /// 带事件发送器创建配置管理器
    pub fn with_event_sender(
        persistence: ConfigPersistence<F>,
        sender: lxx_common::LxxChannelSender<'static, SystemEvent>,
    ) -> Self {
        Self {
            initialized: false,
            config: None,
            event_sender: Some(sender),
            persistence,
        }
    }

    /// 设置事件发送器
    pub async fn set_event_sender(&mut self, sender: lxx_common::LxxChannelSender<'static, SystemEvent>) {
        self.event_sender = Some(sender);
    }

    /// 初始化配置管理器
    pub async fn initialize(&mut self) -> Result<(), lxx_common::SystemError> {
        lxx_common::info!("Initializing config manager");
        self.initialized = true;
        Ok(())
    }

    /// 从存储加载配置
    ///
    /// 如果配置不存在、损坏或版本不匹配，返回默认配置
    pub async fn load_config(&mut self) -> Result<lxx_common::SystemConfig, lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }

        lxx_common::info!("Loading config");

        match self.persistence.load_config::<lxx_common::SystemConfig>().await {
            Ok(config) => {
                lxx_common::info!("Config loaded from storage, version: {}", config.version);
                self.config = Some(config.clone());
                Ok(config)
            }
            Err(e) => {
                lxx_common::warn!("Failed to load config from storage: {:?}, using default config", e);
                let default_config = self.get_default_config();
                self.config = Some(default_config.clone());
                Ok(default_config)
            }
        }
    }

    /// 保存配置到存储
    pub async fn save_config(&mut self, config: lxx_common::SystemConfig) -> Result<(), lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }

        lxx_common::info!("Saving config");

        self.persistence.save_config(&config).await?;

        self.config = Some(config.clone());

        self.notify_config_changed(config);

        Ok(())
    }

    /// 获取当前配置
    pub fn get_config(&self) -> Result<lxx_common::SystemConfig, lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }
        self.config.clone().ok_or_else(|| {
            lxx_common::SystemError::StorageError(lxx_common::StorageError::NotFound)
        })
    }

    /// 更新并保存配置
    pub async fn update_config<U>(&mut self, f: U) -> Result<(), lxx_common::SystemError>
    where
        U: FnOnce(&mut lxx_common::SystemConfig),
    {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }

        let mut config = self.get_config()?;
        f(&mut config);
        self.save_config(config).await
    }

    /// 恢复出厂设置
    pub async fn factory_reset(&mut self) -> Result<(), lxx_common::SystemError> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }

        lxx_common::info!("Performing factory reset");

        self.persistence.factory_reset().await?;

        self.config = None;

        lxx_common::info!("Factory reset completed");

        Ok(())
    }

    /// 通知配置变更
    fn notify_config_changed(&self, _config: lxx_common::SystemConfig) {
        if let Some(ref sender) = self.event_sender {
            for change in [
                ConfigChange::TimeConfig,
                ConfigChange::NetworkConfig,
                ConfigChange::DisplayConfig,
                ConfigChange::PowerConfig,
                ConfigChange::LogConfig,
            ] {
                let event = lxx_common::SystemEvent::ConfigChanged(change);
                let _ = sender.try_send(event);
            }
        }
    }

    /// 获取默认配置
    fn get_default_config(&self) -> lxx_common::SystemConfig {
        lxx_common::SystemConfig {
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
                location_id: heapless::String::new(),
                sync_interval_minutes: 120,
            },
            display_config: lxx_common::DisplayConfig {
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
        }
    }
}