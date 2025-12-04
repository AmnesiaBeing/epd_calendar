// src/driver/time_source/linux.rs

use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use jiff::{Timestamp, Zoned, tz::TimeZone};

use crate::common::error::{AppError, Result};
use crate::driver::time_source::TimeSource;

/// 模拟器RTC时间源 - 模拟ESP32的RTC行为
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct SimulatedRtc {
    // 模拟ESP32 RTC的64位微秒时间戳
    timestamp_us: AtomicU64,
    // 是否已同步（默认false，通过外部接口更新时间后后设为true）
    synchronized: bool,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl SimulatedRtc {
    pub fn new() -> Self {
        let now = Timestamp::now();
        let timestamp_us = now.as_second() * 1_000_000 + now.subsec_microsecond() as i64;
        Self {
            timestamp_us: AtomicU64::new(timestamp_us as u64),
            synchronized: false,
        }
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl TimeSource for SimulatedRtc {
    fn get_time(&self) -> Result<Timestamp> {
        let timestamp_us = self.timestamp_us.load(Ordering::Acquire) as i64;

        let timestamp =
            Timestamp::from_microsecond(timestamp_us as i64).map_err(|_| AppError::TimeError)?;
        log::debug!("Current RTC time: {}", timestamp);
        Ok(timestamp)
    }

    fn set_time(&mut self, new_time: Timestamp) -> Result<()> {
        let timestamp_us = new_time.as_microsecond();
        log::debug!("Setting RTC time to: {}", timestamp_us);
        self.timestamp_us
            .store(timestamp_us as u64, Ordering::Release);
        self.synchronized = true;
        Ok(())
    }
}
