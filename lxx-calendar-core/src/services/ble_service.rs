use lxx_calendar_common::traits::ble::BLEDriver;
use lxx_calendar_common::*;

pub struct BLEService<D: BLEDriver> {
    driver: D,
    timeout_minutes: u32,
    ota_mode: bool,
    enabled: bool,
}

impl<D: BLEDriver> BLEService<D> {
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            timeout_minutes: 5,
            ota_mode: false,
            enabled: true,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing BLE service");
        self.enabled = true;
        info!("BLE service initialized");
        Ok(())
    }

    pub async fn start(&mut self) -> SystemResult<()> {
        if !self.enabled {
            info!("BLE disabled, skipping start");
            return Ok(());
        }

        let is_advertising = self.driver.is_advertising().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })?;

        if is_advertising {
            info!("BLE already advertising");
            return Ok(());
        }

        info!("Starting BLE advertising");

        self.driver.start_advertising().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })?;

        info!("BLE advertising started");
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        let is_advertising = self.driver.is_advertising().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })?;

        let is_connected = self.driver.is_connected().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })?;

        if !is_advertising && !is_connected {
            info!("BLE not advertising or connected");
            return Ok(());
        }

        info!("Stopping BLE");

        self.driver.stop().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })?;

        self.ota_mode = false;

        info!("BLE stopped");
        Ok(())
    }

    pub async fn is_connected(&self) -> SystemResult<bool> {
        self.driver.is_connected().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })
    }

    pub async fn is_advertising(&self) -> SystemResult<bool> {
        self.driver.is_advertising().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })
    }

    pub async fn handle_config(&mut self, data: &[u8]) -> SystemResult<ConfigChange> {
        let is_connected = self.driver.is_connected().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })?;

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
        self.driver.is_configured().map_err(|_| {
            SystemError::HardwareError(HardwareError::InvalidParameter)
        })
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
