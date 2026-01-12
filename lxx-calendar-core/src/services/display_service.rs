use lxx_calendar_common::*;

pub struct DisplayService {
    initialized: bool,
}

impl DisplayService {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing display service");
        self.initialized = true;
        Ok(())
    }

    pub async fn update_display(&mut self, data: DisplayData) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        info!("Updating display");
        Ok(())
    }

    pub async fn refresh(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        info!("Refreshing display");
        Ok(())
    }

    pub async fn get_refresh_state(&self) -> SystemResult<RefreshState> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(RefreshState::Idle)
    }
}
