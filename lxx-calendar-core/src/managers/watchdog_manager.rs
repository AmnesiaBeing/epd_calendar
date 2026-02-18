use lxx_calendar_common::*;

pub struct WatchdogManager<W: Watchdog> {
    wdt: Option<W>,
    initialized: bool,
    timeout_ms: u64,
    enabled: bool,
}

impl<W: Watchdog> WatchdogManager<W> {
    pub fn new(wdt: W) -> Self {
        Self {
            wdt: Some(wdt),
            initialized: false,
            timeout_ms: 30000,
            enabled: true,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing watchdog manager");

        if let Some(ref mut wdt) = self.wdt {
            wdt.enable().map_err(|_| SystemError::HardwareError(HardwareError::NotInitialized))?;
            wdt.set_timeout(self.timeout_ms as u32)
                .map_err(|_| SystemError::HardwareError(HardwareError::NotInitialized))?;
            wdt.feed().map_err(|_| SystemError::HardwareError(HardwareError::NotInitialized))?;
        }

        self.initialized = true;

        info!("Watchdog initialized with {}ms timeout", self.timeout_ms);

        Ok(())
    }

    pub fn feed(&mut self) {
        if !self.initialized || !self.enabled {
            return;
        }

        if let Some(ref mut wdt) = self.wdt {
            wdt.feed().ok();
            debug!("Watchdog fed");
        }
    }

    pub async fn start_task(&mut self) {
        self.feed();
        info!("Watchdog task started");
    }

    pub async fn end_task(&mut self) {
        self.feed();
        info!("Watchdog task ended");
    }

    pub async fn set_timeout(&mut self, timeout_ms: u64) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if timeout_ms < 1000 || timeout_ms > 300000 {
            return Err(SystemError::HardwareError(HardwareError::InvalidParameter));
        }

        self.timeout_ms = timeout_ms;

        if let Some(ref mut wdt) = self.wdt {
            wdt.set_timeout(timeout_ms as u32).ok();
        }

        info!("Watchdog timeout set to {}ms", timeout_ms);

        Ok(())
    }

    pub async fn get_timeout(&self) -> SystemResult<u64> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.timeout_ms)
    }

    pub async fn enable(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        self.enabled = true;

        if let Some(ref mut wdt) = self.wdt {
            wdt.enable().ok();
        }

        info!("Watchdog enabled");

        Ok(())
    }

    pub async fn disable(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        self.enabled = false;

        if let Some(ref mut wdt) = self.wdt {
            wdt.disable().ok();
        }

        info!("Watchdog disabled");

        Ok(())
    }

    pub async fn is_enabled(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.enabled)
    }

    pub async fn is_about_to_timeout(&self) -> SystemResult<bool> {
        if !self.initialized || !self.enabled {
            return Ok(false);
        }

        Ok(false)
    }

    pub async fn check_and_feed(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if self.is_about_to_timeout().await? {
            warn!("Watchdog about to timeout, feeding");
            self.feed();
        }

        Ok(())
    }
}
