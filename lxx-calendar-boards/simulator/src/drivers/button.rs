
use embassy_time::Duration;
use lxx_calendar_common::traits::button::ButtonDriver;

pub struct SimulatorButton;

impl SimulatorButton {
    pub fn new() -> Self {
        Self
    }
}

impl ButtonDriver for SimulatorButton {
    type Error = std::convert::Infallible;

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        info!("Simulator button initialized (not implemented)");
        Ok(())
    }

    async fn wait_for_press(&mut self, timeout: Duration) -> Result<lxx_calendar_common::traits::button::ButtonEvent, Self::Error> {
        // Simulate button press events (mainly for testing)
        // In actual use, button events should be sent by the test harness
        embassy_time::sleep(timeout);

        info!("Simulator button press (mock event)");
        Ok(lxx_calendar_common::traits::button::ButtonEvent::ShortPress)
    }
}
