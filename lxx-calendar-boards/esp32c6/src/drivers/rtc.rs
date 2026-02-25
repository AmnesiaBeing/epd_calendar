use embassy_time::Duration as EmbassyDuration;
use esp_hal::peripherals::Peripherals;
use esp_hal::rtc_cntl::Rtc as EspHalRtc;
use esp_hal::rtc_cntl::sleep::TimerWakeupSource;
use lxx_calendar_common::Rtc;
use lxx_calendar_common::*;

pub struct Esp32Rtc {
    rtc: EspHalRtc<'static>,
    wakeup_source: Option<TimerWakeupSource>,
}

impl Esp32Rtc {
    pub fn new(peripherals: &Peripherals) -> Self {
        Self {
            rtc: esp_hal::rtc_cntl::Rtc::new(unsafe { peripherals.LPWR.clone_unchecked() }),
            wakeup_source: None,
        }
    }
}

impl Rtc for Esp32Rtc {
    type Error = core::convert::Infallible;

    async fn get_time(&self) -> Result<i64, Self::Error> {
        Ok(self.rtc.current_time_us() as i64 / 1_000_000)
    }

    async fn set_time(&mut self, timestamp: i64) -> Result<(), Self::Error> {
        self.rtc.set_current_time_us(timestamp as u64 * 1_000_000);
        info!("ESP32 RTC time set to: {}", timestamp);
        Ok(())
    }

    async fn set_wakeup(&mut self, duration: EmbassyDuration) -> Result<(), Self::Error> {
        let hal_duration = core::time::Duration::from_micros(duration.as_micros() as u64);
        self.wakeup_source = Some(TimerWakeupSource::new(hal_duration));
        info!("ESP32 RTC wakeup set for {:?}", duration);
        Ok(())
    }

    async fn sleep_light(&mut self) {
        if let Some(timer) = self.wakeup_source.take() {
            info!("ESP32 entering light sleep");
            self.rtc.sleep_light(&[&timer]);
        } else {
            info!("No wakeup source set, entering light sleep without timer");
            self.rtc.sleep_light(&[]);
        }
    }
}
