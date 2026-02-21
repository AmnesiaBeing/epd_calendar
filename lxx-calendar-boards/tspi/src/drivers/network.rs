use lxx_calendar_common::NetworkStack;
use lxx_calendar_common::*;

pub struct LinuxNetwork;

impl LinuxNetwork {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStack for LinuxNetwork {
    type Error = core::convert::Infallible;

    fn is_link_up(&self) -> bool {
        true
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        info!("Linux network waiting for config (stub)");
        Ok(())
    }

    fn is_config_up(&self) -> bool {
        true
    }
}
