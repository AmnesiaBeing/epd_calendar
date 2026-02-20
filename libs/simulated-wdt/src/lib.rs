use core::sync::atomic::{AtomicBool, Ordering};
use embassy_executor::{task, Spawner};
use embassy_time::Duration;
use lxx_calendar_common::Watchdog;

static WATCHDOG_FED: AtomicBool = AtomicBool::new(true);
static WATCHDOG_ENABLED: AtomicBool = AtomicBool::new(true);

pub struct SimulatedWdt {
    pub timeout_ms: u64,
}

impl SimulatedWdt {
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

impl Watchdog for SimulatedWdt {
    type Error = core::convert::Infallible;

    fn feed(&mut self) -> Result<(), Self::Error> {
        if WATCHDOG_ENABLED.load(Ordering::SeqCst) {
            WATCHDOG_FED.store(true, Ordering::SeqCst);
            log::debug!("Watchdog fed");
        }
        Ok(())
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        WATCHDOG_ENABLED.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Self::Error> {
        WATCHDOG_ENABLED.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn get_timeout(&self) -> Result<u32, Self::Error> {
        Ok(self.timeout_ms as u32)
    }

    fn set_timeout(&mut self, _timeout_ms: u32) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[task]
async fn watchdog_task(timeout_ms: u64) {
    loop {
        embassy_time::Timer::after(Duration::from_millis(timeout_ms)).await;
        
        if !WATCHDOG_ENABLED.load(Ordering::SeqCst) {
            log::debug!("Watchdog disabled, skipping check");
            continue;
        }
        
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
