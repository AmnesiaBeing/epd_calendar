use esp_hal::{
    peripherals::Peripherals,
    timer::timg::{MwdtStage, TimerGroup, Wdt},
};
use lxx_calendar_common::Watchdog;

pub struct Esp32Watchdog {
    inner: Wdt<esp_hal::peripherals::TIMG0<'static>>,
}

impl Esp32Watchdog {
    pub fn new(peripherals: &Peripherals) -> Self {
        let timg0 = TimerGroup::new(unsafe { peripherals.TIMG0.clone_unchecked() });
        let wdt = timg0.wdt;
        Self { inner: wdt }
    }
}

impl Watchdog for Esp32Watchdog {
    type Error = core::convert::Infallible;

    fn feed(&mut self) -> Result<(), Self::Error> {
        self.inner.feed();
        Ok(())
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        self.inner.enable();
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Self::Error> {
        self.inner.disable();
        Ok(())
    }

    fn get_timeout(&self) -> Result<u32, Self::Error> {
        Ok(0)
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<(), Self::Error> {
        let timeout_us = timeout_ms as u64 * 1000;
        self.inner.set_timeout(
            MwdtStage::Stage0,
            esp_hal::time::Duration::from_micros(timeout_us),
        );
        Ok(())
    }
}
