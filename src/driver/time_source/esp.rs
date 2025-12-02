// src/driver/time_source/esp.rs

#[cfg(feature = "embedded_esp")]
use esp_hal::peripherals::Peripherals;
use esp_hal::rtc_cntl::Rtc;
use jiff::Timestamp;

use crate::common::error::{AppError, Result};
use crate::driver::time_source::TimeSource;

/// ESP32 RTC时间源 - 使用硬件RTC
#[cfg(feature = "embedded_esp")]
pub struct RtcTimeSource {
    // ESP32 RTC实例
    rtc: Rtc<'static>,
    // 是否已同步（默认false，通过外部接口更新时间后后设为true）
    synchronized: bool,
}

#[cfg(feature = "embedded_esp")]
impl RtcTimeSource {
    pub fn new(peripherals: &Peripherals) -> Self {
        log::info!("Initializing RtcTimeSource with hardware RTC");

        let rtc = Rtc::new(unsafe { peripherals.LPWR.clone_unchecked() });

        Self {
            rtc,
            synchronized: false,
        }
    }
}

#[cfg(feature = "embedded_esp")]
impl TimeSource for RtcTimeSource {
    fn get_time(&self) -> Result<Timestamp> {
        let timestamp_us = self.rtc.current_time_us();
        let timestamp =
            Timestamp::from_microsecond(timestamp_us as i64).map_err(|_| AppError::TimeError)?;
        log::debug!("Current RTC time: {}", timestamp);
        Ok(timestamp)
    }

    fn set_time(&mut self, new_time: Timestamp) -> Result<()> {
        let timestamp_us = new_time.as_microsecond();
        log::debug!("Setting RTC time to: {}", timestamp_us);
        self.rtc.set_current_time_us(timestamp_us as u64);
        Ok(())
    }
}
