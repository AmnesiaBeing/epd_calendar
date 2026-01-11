use lxx_calendar_common as lxxcc;
use lxxcc::{SystemResult, SystemError};

pub struct AudioService {
    initialized: bool,
}

impl AudioService {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        lxxcc::info!("Initializing audio service");
        self.initialized = true;
        Ok(())
    }

    pub async fn play_hour_chime(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Playing hour chime");
        Ok(())
    }

    pub async fn play_alarm(&mut self, melody: lxxcc::Melody) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Playing alarm: {:?}", melody);
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Stopping audio");
        Ok(())
    }
}
