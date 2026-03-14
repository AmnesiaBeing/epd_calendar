use core::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use embassy_time::{Duration, Instant};
use lxx_calendar_common::{Rtc, info};
use std::sync::Arc;

static RTC_TIMESTAMP: AtomicI64 = AtomicI64::new(1771588453);

pub struct SimulatedRtc {
    initialized: bool,
    base_timestamp: i64,
    boot_instant: Option<Instant>,
    wakeup_duration: Option<Duration>,
    wakeup_flag: Arc<AtomicBool>,
}

impl SimulatedRtc {
    pub fn new() -> Self {
        Self {
            initialized: false,
            base_timestamp: 1771588453,
            boot_instant: None,
            wakeup_duration: None,
            wakeup_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_wakeup_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.wakeup_flag)
    }

    pub fn request_wakeup(&self) {
        self.wakeup_flag.store(true, Ordering::SeqCst);
        info!("Wakeup requested (button pressed)");
    }

    pub fn clear_wakeup_flag(&self) {
        self.wakeup_flag.store(false, Ordering::SeqCst);
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
        info!(
            "Simulated RTC initialized with base timestamp: {}",
            self.base_timestamp
        );
        Ok(())
    }

    pub fn get_timestamp(&self) -> i64 {
        if let Some(instant) = self.boot_instant {
            let elapsed = instant.elapsed().as_secs() as i64;
            self.base_timestamp + elapsed
        } else {
            self.base_timestamp
        }
    }

    pub fn set_timestamp(&mut self, timestamp: i64) {
        self.base_timestamp = timestamp;
        RTC_TIMESTAMP.store(timestamp, Ordering::SeqCst);
        self.boot_instant = Some(Instant::now());
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
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
        info!("Simulated RTC time set to: {}", timestamp);
        Ok(())
    }

    async fn set_wakeup(&mut self, duration: Duration) -> Result<(), Self::Error> {
        self.wakeup_duration = Some(duration);
        info!("Simulated RTC wakeup set for {:?}", duration);
        Ok(())
    }

    async fn sleep_light(&mut self) {
        if let Some(duration) = self.wakeup_duration.take() {
            let start = Instant::now();

            info!("Simulated light sleep for {:?}", duration);

            // 每 100ms 检查一次 wakeup flag，模拟硬件中断唤醒
            while start.elapsed() < duration {
                if self.wakeup_flag.load(Ordering::SeqCst) {
                    self.wakeup_flag.store(false, Ordering::SeqCst);
                    info!("Woke up early (button pressed)");
                    break;
                }
                embassy_time::block_for(Duration::from_millis(100));
            }

            if start.elapsed() >= duration {
                info!("Simulated wakeup from light sleep (timeout)");
            }
        } else {
            info!("Simulated light sleep (no wakeup duration set)");
        }
    }
}
