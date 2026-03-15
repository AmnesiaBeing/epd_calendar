use core::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use embassy_time::{Duration, Instant};
use lxx_calendar_common::{Rtc, info};
use std::sync::{Arc, Condvar, Mutex};

static RTC_TIMESTAMP: AtomicI64 = AtomicI64::new(1771588453);

/// 共享的睡眠状态，用于 Condvar 等待
#[derive(Clone)]
pub struct SleepState {
    should_wakeup: Arc<(Mutex<bool>, Condvar)>,
}

impl SleepState {
    pub fn new() -> Self {
        Self {
            should_wakeup: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    pub fn new_with_condvar(condvar: Arc<(Mutex<bool>, Condvar)>) -> Self {
        Self {
            should_wakeup: condvar,
        }
    }

    pub fn get_condvar(&self) -> Arc<(Mutex<bool>, Condvar)> {
        Arc::clone(&self.should_wakeup)
    }

    pub fn request_wakeup(&self) {
        let (lock, cvar) = &*self.should_wakeup;
        let mut should_wakeup = lock.lock().unwrap();
        *should_wakeup = true;
        cvar.notify_one();
        info!("Wakeup requested (button pressed)");
    }

    pub fn clear_wakeup_flag(&self) {
        let (lock, _) = &*self.should_wakeup;
        let mut should_wakeup = lock.lock().unwrap();
        *should_wakeup = false;
    }

    pub fn wait_for_wakeup(&self, duration: Duration) -> bool {
        let (lock, cvar) = &*self.should_wakeup;

        // 等待指定时间或被唤醒
        let mut should_wakeup = lock.lock().unwrap();
        let mut result = cvar.wait_timeout(should_wakeup, duration.into()).unwrap();

        let woke_up = *result.0;

        // 清除唤醒标志
        *result.0 = false;

        woke_up
    }
}

impl Default for SleepState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct SimulatedRtc {
    initialized: bool,
    base_timestamp: i64,
    boot_instant: Option<Instant>,
    wakeup_duration: Option<Duration>,
    sleep_state: SleepState,
}

impl SimulatedRtc {
    pub fn new() -> Self {
        Self {
            initialized: false,
            base_timestamp: 1771588453,
            boot_instant: None,
            wakeup_duration: None,
            sleep_state: SleepState::new(),
        }
    }

    pub fn get_sleep_state(&self) -> SleepState {
        self.sleep_state.clone()
    }

    pub fn request_wakeup(&self) {
        self.sleep_state.request_wakeup();
    }

    pub fn clear_wakeup_flag(&self) {
        self.sleep_state.clear_wakeup_flag();
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
            info!("Simulated light sleep for {:?}", duration);

            // 使用 Condvar 等待，不会阻塞 HTTP 服务器线程
            let woke_up = self.sleep_state.wait_for_wakeup(duration);

            if woke_up {
                info!("Woke up early (button pressed)");
            } else {
                info!("Simulated wakeup from light sleep (timeout)");
            }
        } else {
            info!("Simulated light sleep (no wakeup duration set)");
        }
    }
}
