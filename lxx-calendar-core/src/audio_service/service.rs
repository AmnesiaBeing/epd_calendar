use lxx_calendar_common as lxx_common;
use lxx_common::{SystemResult, SystemError};

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
        lxx_common::info!("Initializing audio service");
        self.initialized = true;
        Ok(())
    }

    pub async fn play_hour_chime(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        lxx_common::info!("Playing hour chime");
        Ok(())
    }

    pub async fn play_alarm(&mut self, melody: lxx_common::Melody) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        lxx_common::info!("Playing alarm: {:?}", melody);
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        lxx_common::info!("Stopping audio");
        Ok(())
    }
}
