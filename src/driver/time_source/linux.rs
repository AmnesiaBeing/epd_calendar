// src/driver/time_source/linux.rs

use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use jiff::{Timestamp, Zoned, tz::TimeZone};

use crate::common::error::{AppError, Result};
use crate::driver::ntp_source::SntpSource;

/// 模拟器RTC时间源 - 模拟ESP32的RTC行为
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct SimulatedRtc {
    // 模拟ESP32 RTC的64位微秒时间戳
    timestamp_us: AtomicU64,
    // 起始时间点
    start_time: Instant,
    // NTP时间源
    ntp_time_source: SntpSource,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl SimulatedRtc {
    pub fn new(ntp_time_source: SntpSource) -> Self {
        let now = Timestamp::now();
        let timestamp_us = now.as_second() * 1_000_000 + now.subsec_microsecond() as i64;
        Self {
            timestamp_us: AtomicU64::new(timestamp_us as u64),
            start_time: Instant::now(),
            ntp_time_source,
        }
    }

    /// 更新时间戳（内部方法）
    fn update_timestamp(&self, new_timestamp: Timestamp) {
        let timestamp_us =
            new_timestamp.as_second() * 1_000_000 + new_timestamp.subsec_microsecond() as i64;
        self.timestamp_us
            .store(timestamp_us as u64, Ordering::Release);
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl TimeSource for SimulatedRtc {
    fn get_time(&self) -> Result<Zoned> {
        let timestamp_us = self.timestamp_us.load(Ordering::Acquire) as i64;

        let utc = Zoned::new(
            Timestamp::from_microsecond(timestamp_us).unwrap(),
            TimeZone::UTC,
        );

        Ok(utc)
    }

    fn update_time_by_sntp(&mut self) -> Result<()> {
        match self.ntp_time_source.sync_time().await {
            Ok(ntp_time) => {
                // 更新RTC时间戳
                self.update_timestamp(ntp_time);
                self.start_time = Instant::now(); // 重置起始时间
                Ok(())
            }
            Err(e) => {
                log::error!("SNTP time update failed: {:?}", e);
                Err(AppError::TimeError)
            }
        }
    }

    fn get_timestamp_us(&self) -> Result<u64> {
        let elapsed = self.start_time.elapsed().as_micros() as u64;
        let base_timestamp = self.timestamp_us.load(Ordering::Acquire);
        Ok(base_timestamp + elapsed)
    }
}
