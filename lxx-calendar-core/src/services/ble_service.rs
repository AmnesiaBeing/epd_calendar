use alloc::boxed::Box;
use lxx_calendar_common::events::BLEEvent;
use lxx_calendar_common::traits::ble::BLEDriver;
use lxx_calendar_common::*;

pub struct BLEService<D: BLEDriver> {
    driver: D,
    timeout_minutes: u32,
    ota_mode: bool,
    enabled: bool,
    event_sender: Option<LxxChannelSender<'static, SystemEvent>>,
}

impl<D: BLEDriver> BLEService<D> {
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            timeout_minutes: 5,
            ota_mode: false,
            enabled: true,
            event_sender: None,
        }
    }

    pub async fn initialize(
        &mut self,
        sender: LxxChannelSender<'static, SystemEvent>,
    ) -> SystemResult<()> {
        info!("Initializing BLE service");

        self.event_sender = Some(sender.clone());

        let sender_clone = sender;
        self.driver.set_data_callback(Box::new(move |data| {
            if let Some(ble_event) = parse_ble_event(data) {
                let event = SystemEvent::BLEEvent(ble_event);
                let _ = sender_clone.try_send(event);
            }
        }));

        self.enabled = true;
        info!("BLE service initialized");
        Ok(())
    }

    pub async fn start(&mut self) -> SystemResult<()> {
        if !self.enabled {
            info!("BLE disabled, skipping start");
            return Ok(());
        }

        let is_advertising = self
            .driver
            .is_advertising()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))?;

        if is_advertising {
            info!("BLE already advertising");
            return Ok(());
        }

        info!("Starting BLE advertising");

        self.driver
            .start_advertising()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))?;

        info!("BLE advertising started");
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        let is_advertising = self
            .driver
            .is_advertising()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))?;

        let is_connected = self
            .driver
            .is_connected()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))?;

        if !is_advertising && !is_connected {
            info!("BLE not advertising or connected");
            return Ok(());
        }

        info!("Stopping BLE");

        self.driver
            .stop()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))?;

        self.ota_mode = false;

        info!("BLE stopped");
        Ok(())
    }

    pub async fn is_connected(&self) -> SystemResult<bool> {
        self.driver
            .is_connected()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))
    }

    pub async fn is_advertising(&self) -> SystemResult<bool> {
        self.driver
            .is_advertising()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))
    }

    pub async fn handle_config(&mut self, data: &[u8]) -> SystemResult<ConfigChange> {
        let is_connected = self
            .driver
            .is_connected()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))?;

        if !is_connected {
            return Err(SystemError::ServiceError(ServiceError::InvalidState));
        }

        info!("Processing BLE config data ({} bytes)", data.len());

        let change = self.parse_config_data(data)?;

        info!("Config processed: {:?}", change);

        Ok(change)
    }

    fn parse_config_data(&self, data: &[u8]) -> SystemResult<ConfigChange> {
        if data.len() < 10 {
            Ok(ConfigChange::TimeConfig)
        } else if data.len() < 50 {
            Ok(ConfigChange::NetworkConfig)
        } else {
            Ok(ConfigChange::DisplayConfig)
        }
    }

    pub async fn start_ota(&mut self) -> SystemResult<()> {
        info!("Starting OTA mode");
        self.ota_mode = true;
        info!("OTA mode started");
        Ok(())
    }

    pub async fn receive_firmware(&mut self, data: &[u8]) -> SystemResult<()> {
        if !self.ota_mode {
            return Err(SystemError::HardwareError(HardwareError::InvalidParameter));
        }

        info!("Receiving firmware data ({} bytes)", data.len());
        Ok(())
    }

    pub async fn finish_ota(&mut self) -> SystemResult<()> {
        if !self.ota_mode {
            return Err(SystemError::HardwareError(HardwareError::InvalidParameter));
        }

        info!("Finishing OTA");
        self.ota_mode = false;
        info!("OTA completed, ready to reboot");
        Ok(())
    }

    pub async fn cancel_ota(&mut self) -> SystemResult<()> {
        info!("Canceling OTA");
        self.ota_mode = false;
        Ok(())
    }

    pub async fn is_configured(&self) -> SystemResult<bool> {
        self.driver
            .is_configured()
            .map_err(|_| SystemError::HardwareError(HardwareError::InvalidParameter))
    }

    pub async fn set_timeout(&mut self, minutes: u32) -> SystemResult<()> {
        self.timeout_minutes = minutes;
        info!("BLE timeout set to {} minutes", minutes);
        Ok(())
    }

    pub async fn set_enabled(&mut self, enabled: bool) -> SystemResult<()> {
        self.enabled = enabled;

        if !enabled {
            self.stop().await?;
        }

        info!("BLE enabled: {}", enabled);
        Ok(())
    }

    pub async fn enter_pairing_mode(&mut self) -> SystemResult<()> {
        info!("Entering BLE pairing mode");
        self.start().await?;
        Ok(())
    }

    pub async fn exit_pairing_mode(&mut self) -> SystemResult<()> {
        info!("Exiting BLE pairing mode");
        self.stop().await?;
        Ok(())
    }

    pub async fn get_device_name(&self) -> SystemResult<heapless::String<32>> {
        Ok(heapless::String::try_from("LXX-Calendar").unwrap_or_default())
    }
}

fn parse_ble_event(data: &[u8]) -> Option<BLEEvent> {
    let json: serde_json::Value = serde_json::from_slice(data).ok()?;
    let msg_type = json.get("type")?.as_str()?;
    let data_obj = json.get("data")?;

    match msg_type {
        "wifi_config" => {
            let ssid = data_obj.get("wifi_ssid")?.as_str()?;
            let password = data_obj.get("wifi_password")?.as_str()?;
            Some(BLEEvent::WifiConfigReceived {
                ssid: heapless::String::try_from(ssid).unwrap_or_default(),
                password: heapless::String::try_from(password).unwrap_or_default(),
            })
        }
        "network_config" => {
            let location_id = data_obj.get("location_id")?.as_str()?;
            let sync_interval_minutes = data_obj.get("sync_interval_minutes")?.as_u64()? as u16;
            let auto_sync = data_obj
                .get("auto_sync")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            Some(BLEEvent::NetworkConfigReceived {
                location_id: heapless::String::try_from(location_id).unwrap_or_default(),
                sync_interval_minutes,
                auto_sync,
            })
        }
        "display_config" => {
            let refresh_interval_seconds =
                data_obj.get("refresh_interval_seconds")?.as_u64()? as u16;
            let low_power_refresh_enabled = data_obj.get("low_power_refresh_enabled")?.as_bool()?;
            Some(BLEEvent::DisplayConfigReceived {
                refresh_interval_seconds,
                low_power_refresh_enabled,
            })
        }
        "time_config" => {
            let timezone_offset = data_obj.get("timezone_offset")?.as_i64()? as i32;
            let hour_chime_enabled = data_obj
                .get("hour_chime_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            Some(BLEEvent::TimeConfigReceived {
                timezone_offset,
                hour_chime_enabled,
            })
        }
        "power_config" => {
            let low_power_mode_enabled = data_obj.get("low_power_mode_enabled")?.as_bool()?;
            Some(BLEEvent::PowerConfigReceived {
                low_power_mode_enabled,
            })
        }
        "log_config" => {
            let log_level = match data_obj.get("log_level")?.as_str()? {
                "error" => LogLevel::Error,
                "warn" => LogLevel::Warn,
                "info" => LogLevel::Info,
                "debug" => LogLevel::Debug,
                "trace" => LogLevel::Trace,
                _ => LogLevel::Info,
            };
            let log_to_flash = data_obj
                .get("log_to_flash")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            Some(BLEEvent::LogConfigReceived {
                log_level,
                log_to_flash,
            })
        }
        "command" => {
            let action = data_obj.get("action")?.as_str()?;
            match action {
                "network_sync" => Some(BLEEvent::CommandNetworkSync),
                "reboot" => Some(BLEEvent::CommandReboot),
                "factory_reset" => Some(BLEEvent::CommandFactoryReset),
                _ => None,
            }
        }
        "ota_start" => Some(BLEEvent::OTAStart),
        "ota_data" => {
            let data_bytes = data_obj.get("data")?.as_array()?;
            let mut vec = heapless::Vec::new();
            for byte in data_bytes {
                let b = byte.as_u64()? as u8;
                vec.push(b).ok()?;
            }
            Some(BLEEvent::OTAData(vec))
        }
        "ota_complete" => Some(BLEEvent::OTAComplete),
        "ota_cancel" => Some(BLEEvent::OTACancel),
        _ => None,
    }
}
