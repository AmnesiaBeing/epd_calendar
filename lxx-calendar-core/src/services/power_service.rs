use lxx_calendar_common::*;

pub struct PowerManager {
    initialized: bool,
    battery_level: u8,
    charging: bool,
    low_power_mode: bool,
    low_battery_threshold: u8,
    critical_battery_threshold: u8,
    calibration_offset: i32,
    last_calibration_time: Option<u64>,
}

impl PowerManager {
    pub fn new() -> Self {
        Self {
            initialized: false,
            battery_level: 100,
            charging: false,
            low_power_mode: false,
            low_battery_threshold: 30,
            critical_battery_threshold: 10,
            calibration_offset: 0,
            last_calibration_time: None,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing power manager");

        self.battery_level = self.read_battery_level().await?;
        self.charging = self.check_charging_status().await?;
        self.update_power_mode();

        self.initialized = true;

        info!(
            "Power manager initialized, battery: {}%",
            self.battery_level
        );

        Ok(())
    }

    async fn read_battery_level(&self) -> SystemResult<u8> {
        // 预留ADC电池电压检测接口
        // 实际实现需要读取ESP32-C6 ADC
        // 返回模拟值用于测试
        Ok(75)
    }

    async fn check_charging_status(&self) -> SystemResult<bool> {
        // 预留充电状态检测接口
        // 实际实现需要读取GPIO状态
        Ok(false)
    }

    fn update_power_mode(&mut self) {
        let was_low_power = self.low_power_mode;

        if self.battery_level < self.low_battery_threshold {
            self.low_power_mode = true;
        } else if self.battery_level >= self.low_battery_threshold + 5 {
            self.low_power_mode = false;
        }

        if was_low_power != self.low_power_mode {
            info!("Power mode changed: low_power_mode={}", self.low_power_mode);
        }
    }

    pub async fn get_battery_level(&self) -> SystemResult<u8> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.battery_level)
    }

    pub async fn refresh_battery_level(&mut self) -> SystemResult<u8> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let raw_level = self.read_battery_level().await?;
        let calibrated_level = ((raw_level as i32) + self.calibration_offset).clamp(0, 100) as u8;

        let was_low_power = self.low_power_mode;
        self.battery_level = calibrated_level;
        self.charging = self.check_charging_status().await?;

        self.update_power_mode();

        if was_low_power != self.low_power_mode {
            info!("Power state changed due to battery update");
        }

        Ok(self.battery_level)
    }

    pub async fn is_low_battery(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.low_power_mode)
    }

    pub async fn is_critical_battery(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.battery_level < self.critical_battery_threshold)
    }

    pub async fn is_charging(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.charging)
    }

    pub async fn enter_low_power_mode(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if !self.low_power_mode {
            self.low_power_mode = true;
            warn!("Entering low power mode manually");
        }

        Ok(())
    }

    pub async fn exit_low_power_mode(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if self.low_power_mode && self.battery_level >= self.low_battery_threshold {
            self.low_power_mode = false;
            info!("Exiting low power mode");
        }

        Ok(())
    }

    pub async fn set_low_battery_threshold(&mut self, threshold: u8) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if threshold > 100 {
            return Err(SystemError::HardwareError(HardwareError::InvalidParameter));
        }

        self.low_battery_threshold = threshold;
        self.update_power_mode();

        info!("Low battery threshold set to {}%", threshold);

        Ok(())
    }

    pub async fn set_critical_battery_threshold(&mut self, threshold: u8) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if threshold > 100 || threshold >= self.low_battery_threshold {
            return Err(SystemError::HardwareError(HardwareError::InvalidParameter));
        }

        self.critical_battery_threshold = threshold;

        info!("Critical battery threshold set to {}%", threshold);

        Ok(())
    }

    pub async fn calibrate(&mut self, actual_level: u8) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let current_raw = self.read_battery_level().await?;
        self.calibration_offset = (actual_level as i32) - (current_raw as i32);
        self.last_calibration_time = Some(embassy_time::Instant::now().elapsed().as_secs());

        info!("Battery calibrated: offset={}", self.calibration_offset);

        Ok(())
    }

    pub async fn get_power_status(&self) -> SystemResult<PowerStatus> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        Ok(PowerStatus {
            battery_level: self.battery_level,
            charging: self.charging,
            low_power_mode: self.low_power_mode,
            critical: self.battery_level < self.critical_battery_threshold,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PowerStatus {
    pub battery_level: u8,
    pub charging: bool,
    pub low_power_mode: bool,
    pub critical: bool,
}
