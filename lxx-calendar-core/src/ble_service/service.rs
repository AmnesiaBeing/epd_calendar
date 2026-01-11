use lxx_calendar_common as lxxcc;
use lxxcc::{SystemResult, SystemError};

pub struct BLEService {
    initialized: bool,
}

impl BLEService {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), lxxcc::SystemError> {
        lxxcc::info!("Initializing BLE service");
        self.initialized = true;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), lxxcc::SystemError> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Starting BLE");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), lxxcc::SystemError> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Stopping BLE");
        Ok(())
    }

    pub async fn is_connected(&self) -> Result<bool, lxxcc::SystemError> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        Ok(false)
    }
}
