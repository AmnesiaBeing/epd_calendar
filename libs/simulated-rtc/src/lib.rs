use core::sync::atomic::{AtomicI64, Ordering};
use embassy_time::Instant;
use lxx_calendar_common::Rtc;

static RTC_TIMESTAMP: AtomicI64 = AtomicI64::new(1704067200);

pub struct SimulatedRtc {
    initialized: bool,
    base_timestamp: i64,
    boot_instant: Option<Instant>,
}

impl SimulatedRtc {
    pub fn new() -> Self {
        Self {
            initialized: false,
            base_timestamp: 1704067200,
            boot_instant: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), core::convert::Infallible> {
        let stored = RTC_TIMESTAMP.load(Ordering::SeqCst);
        if stored > 0 {
            self.base_timestamp = stored;
        } else {
            self.base_timestamp = 1704067200;
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
}
