use embassy_executor::{Spawner, task};
use embassy_time::Duration;
use core::sync::atomic::{AtomicBool, Ordering};

static WATCHDOG_FED: AtomicBool = AtomicBool::new(true);

pub struct SimulatedWdt {
    timeout_ms: u64,
}

impl SimulatedWdt {
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }

    pub fn feed(&self) {
        WATCHDOG_FED.store(true, Ordering::SeqCst);
        log::debug!("Watchdog fed");
    }

    pub fn is_armed(&self) -> bool {
        WATCHDOG_FED.load(Ordering::SeqCst)
    }
}

#[task]
async fn watchdog_task(timeout_ms: u64) {
    loop {
        embassy_time::Timer::after(Duration::from_millis(timeout_ms)).await;
        if !WATCHDOG_FED.load(Ordering::SeqCst) {
            log::warn!("Watchdog expired!");
        } else {
            log::debug!("Watchdog check: fed");
        }
        WATCHDOG_FED.store(false, Ordering::SeqCst);
    }
}

pub fn start_watchdog(spawner: &Spawner, timeout_ms: u64) {
    spawner.spawn(watchdog_task(timeout_ms)).ok();
}

pub struct NoopWdt;

impl NoopWdt {
    pub fn new() -> Self {
        Self
    }

    pub fn feed(&self) {}
}

impl Default for NoopWdt {
    fn default() -> Self {
        Self::new()
    }
}
