use lxx_calendar_common::NetworkStack;
use lxx_calendar_common::*;

pub struct SimulatorNetwork;

impl SimulatorNetwork {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatorNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStack for SimulatorNetwork {
    type Error = core::convert::Infallible;

    fn is_link_up(&self) -> bool {
        false
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        info!("[Simulator Network] Waiting for config (stub)");
        Ok(())
    }

    fn is_config_up(&self) -> bool {
        false
    }
}
