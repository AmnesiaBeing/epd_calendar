use log::info;
use lxx_calendar_common::traits::button::{ButtonDriver, ButtonEvent};

pub struct SimulatorButton;

impl SimulatorButton {
    pub fn new() -> Self {
        Self
    }
}

impl ButtonDriver for SimulatorButton {
    type Error = std::convert::Infallible;

    async fn register_press_callback<F>(&mut self, _callback: F) -> Result<(), Self::Error>
    where
        F: Fn(ButtonEvent) + Send + 'static,
    {
        info!("Simulator button callback registered (not implemented)");
        Ok(())
    }
}
