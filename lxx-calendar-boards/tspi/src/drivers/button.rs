use lxx_calendar_common::traits::button::{ButtonDriver, ButtonEvent};
use lxx_calendar_common::*;

pub struct TspiButton;

impl TspiButton {
    pub fn new() -> Self {
        Self
    }
}

impl ButtonDriver for TspiButton {
    type Error = std::io::Error;

    async fn register_press_callback<F>(&mut self, _callback: F) -> Result<(), Self::Error>
    where
        F: Fn(ButtonEvent) + Send + 'static,
    {
        info!("TSPI button callback registered (not implemented)");
        Ok(())
    }
}
