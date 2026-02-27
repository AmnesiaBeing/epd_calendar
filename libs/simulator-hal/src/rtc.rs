use core::sync::atomic::{AtomicI64, Ordering};
use embassy_time::{Duration, Instant};
use lxx_calendar_common::Rtc;

static RTC_TIMESTAMP: AtomicI64 = AtomicI64::new(1771588453);

pub struct SimulatedRtc {
    initialized: bool,
    base_timestamp: i64,
    boot_instant: Option<Instant>,
    wakeup_duration: Option<Duration>,
}

impl SimulatedRtc {
    pub fn new() -> Self {
        Self {
            initialized: false,
            base_timestamp: 1771588453,
            boot_instant: None,
            wakeup_duration: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), core::convert::Infallible> {
        let stored = RTC_TIMESTAMP.load(Ordering::SeqCst);
        if stored > 0 {
            self.base_timestamp = stored;
        } else {
            self.base_timestamp = 1771588453;
        }
        self.boot_instant = Some(Instant::now());
        self.initialized = true;
        log::info!(
            "Simulated RTC initialized with base timestamp: {}",
            self.base_timestamp
        );
        Ok(())
    }
}

impl Default for SimulatedRtc {
    fn default() -> Self {
        Self::new()
    }
}

impl Rtc for SimulatedRtc {
    type Error = core::convert::Infallible;

    async fn get_time(&self) -> Result<i64, Self::Error> {
        if !self.initialized {
            return Ok(0);
        }

        if let Some(instant) = self.boot_instant {
            let elapsed = instant.elapsed().as_secs() as i64;
            Ok(self.base_timestamp + elapsed)
        } else {
            Ok(self.base_timestamp)
        }
    }

    async fn set_time(&mut self, timestamp: i64) -> Result<(), Self::Error> {
        self.base_timestamp = timestamp;
        RTC_TIMESTAMP.store(timestamp, Ordering::SeqCst);
        self.boot_instant = Some(Instant::now());
        log::info!("Simulated RTC time set to: {}", timestamp);
        Ok(())
    }

    async fn set_wakeup(&mut self, duration: Duration) -> Result<(), Self::Error> {
        self.wakeup_duration = Some(duration);
        log::info!("Simulated RTC wakeup set for {:?}", duration);
        Ok(())
    }

    async fn sleep_light(&mut self) {
        if let Some(duration) = self.wakeup_duration.take() {
            log::info!("Simulated light sleep for {:?}", duration);
            embassy_time::block_for(duration);
            log::info!("Simulated wakeup from light sleep");
        } else {
            log::info!("Simulated light sleep (no wakeup duration set)");
        }
    }
}
