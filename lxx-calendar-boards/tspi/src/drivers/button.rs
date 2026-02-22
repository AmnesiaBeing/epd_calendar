
use embassy_time::Duration;
use lxx_calendar_common::traits::button::ButtonDriver;

pub struct TspiButton;

impl TspiButton {
    pub fn new() -> Self {
        Self
    }
}

impl ButtonDriver for TspiButton {
    type Error = std::io::Error;

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        info!("TSPI button initialized (not implemented)");
        Ok(())
    }

    async fn wait_for_press(&mut self, timeout: Duration) -> Result<lxx_calendar_common::traits::button::ButtonEvent, Self::Error> {
        // TODO: Implement button reading based on Linux event subsystem
        // For now, return a mock event (mainly for testing)

        embassy_time::sleep(timeout);

        info!("TSPI button press (not implemented)");
        Ok(lxx_calendar_common::traits::button::ButtonEvent::ShortPress)
    }
}
