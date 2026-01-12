use lxx_calendar_common::*;

pub struct AudioService {
    initialized: bool,
}

impl AudioService {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing audio service");
        self.initialized = true;
        Ok(())
    }

    pub async fn play_hour_chime(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(
                HardwareError::NotInitialized,
            ));
        }
        info!("Playing hour chime");
        Ok(())
    }

    pub async fn play_alarm(&mut self, melody: Melody) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(
                HardwareError::NotInitialized,
            ));
        }
        info!("Playing alarm: {:?}", melody);
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(
                HardwareError::NotInitialized,
            ));
        }
        info!("Stopping audio");
        Ok(())
    }
}
