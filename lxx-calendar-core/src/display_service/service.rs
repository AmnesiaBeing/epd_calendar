use lxx_calendar_common as lxxcc;
use lxxcc::{SystemResult, SystemError};

pub struct DisplayService {
    initialized: bool,
}

impl DisplayService {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        lxxcc::info!("Initializing display service");
        self.initialized = true;
        Ok(())
    }

    pub async fn update_display(&mut self, data: lxxcc::DisplayData) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Updating display");
        Ok(())
    }

    pub async fn refresh(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Refreshing display");
        Ok(())
    }

    pub async fn get_refresh_state(&self) -> SystemResult<lxxcc::RefreshState> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        Ok(lxxcc::RefreshState::Idle)
    }
}
