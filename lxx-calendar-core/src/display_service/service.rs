use lxx_calendar_common as lxx_common;
use lxx_common::{SystemError, SystemResult};

pub struct DisplayService {
    initialized: bool,
}

impl DisplayService {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        lxx_common::info!("Initializing display service");
        self.initialized = true;
        Ok(())
    }

    pub async fn update_display(&mut self, data: lxx_common::DisplayData) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }
        lxx_common::info!("Updating display");
        Ok(())
    }

    pub async fn refresh(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }
        lxx_common::info!("Refreshing display");
        Ok(())
    }

    pub async fn get_refresh_state(&self) -> SystemResult<lxx_common::RefreshState> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::NotInitialized,
            ));
        }
        Ok(lxx_common::RefreshState::Idle)
    }
}
