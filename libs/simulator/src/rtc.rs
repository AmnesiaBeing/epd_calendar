use core::sync::atomic::AtomicI64;
use core::sync::atomic::Ordering;
use embassy_time::{Duration, Instant};
use lxx_calendar_common::{Rtc, info};
use std::sync::{Arc, Mutex};

static RTC_TIMESTAMP: AtomicI64 = AtomicI64::new(1771588453);

/// 共享的睡眠状态，用于轮询等待
#[derive(Clone)]
pub struct SleepState {
    should_wakeup: Arc<Mutex<bool>>,
}

impl SleepState {
    pub fn new() -> Self {
        Self {
            should_wakeup: Arc::new(Mutex::new(false)),
        }
    }

    pub fn request_wakeup(&self) {
        let mut should_wakeup = self.should_wakeup.lock().unwrap();
        *should_wakeup = true;
        info!("Wakeup requested (button pressed)");
    }

    pub fn clear_wakeup_flag(&self) {
        let mut should_wakeup = self.should_wakeup.lock().unwrap();
        *should_wakeup = false;
    }

    pub fn get_flag(&self) -> Arc<Mutex<bool>> {
        Arc::clone(&self.should_wakeup)
    }

    /// 异步等待唤醒或超时
    pub async fn wait_for_wakeup(&self, duration: Duration) -> bool {
        use embassy_time::{Timer, Duration};
        
        // 轮询检查唤醒标志，同时计时
        let start = Instant::now();
        loop {
            // 检查是否被唤醒
            {
                let should_wakeup = self.should_wakeup.lock().unwrap();
                if *should_wakeup {
                    drop(should_wakeup);
                    // 清除唤醒标志
                    self.clear_wakeup_flag();
                    return true;
                }
            }
            
            // 检查是否超时
            if start.elapsed() >= duration {
                return false;
            }
            
            // 短暂等待后继续轮询（100ms）
            Timer::after(Duration::from_millis(100)).await;
        }
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
}
