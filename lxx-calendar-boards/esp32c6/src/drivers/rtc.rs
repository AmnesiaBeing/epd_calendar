use lxx_calendar_common::Rtc;
use lxx_calendar_common::*;

pub struct Esp32Rtc;

impl Esp32Rtc {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Esp32Rtc {
    fn default() -> Self {
        Self::new()
    }
}

impl Rtc for Esp32Rtc {
    type Error = core::convert::Infallible;

    async fn get_time(&self) -> Result<i64, Self::Error> {
        Ok(0)
    }

    async fn set_time(&mut self, timestamp: i64) -> Result<(), Self::Error> {
        info!("ESP32 RTC time set to: {}", timestamp);
        Ok(())
    }
}
