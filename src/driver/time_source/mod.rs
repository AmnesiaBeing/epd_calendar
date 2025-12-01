// src/driver/time_source.rs
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

#[cfg(feature = "embedded_esp")]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(feature = "embedded_esp")]
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
#[cfg(feature = "embedded_esp")]
use esp_hal::{
    peripherals::{LPWR, Peripherals},
    rtc_cntl::Rtc,
};
use jiff::{Timestamp, Zoned, tz::TimeZone};

use crate::common::error::{AppError, Result};
#[cfg(feature = "embedded_esp")]
use crate::driver::network::NetworkDriver;
use crate::driver::ntp_source::SntpSource;

// 时间逻辑声明
// ESP32内部实际上使用两个u32的RTC寄存器存储时间，通过调用pub fn set_current_time_us(&self, current_time_us: u64)函数来写入寄存器
// ESP32可通过pub fn current_time_us(&self) -> u64来读取当前时间
// 模拟器内使用1个u64来存储时间
// 存储的时间类型均为Timestamp（u64），时区相关信息不在本代码做处理

// 关于SNTP和时间源的关系
// ESP32内，通过SNTP更新时间后（SNTP是外部调用的），会调用pub fn set_current_time_us(&self, current_time_us: u64)函数来写入寄存器
// 模拟器内，通过SNTP更新时间后，会调用SimulatedRtc::update_timestamp方法来更新时间戳

// 时间中断逻辑声明
// ESP32可以通过pub fn set_interrupt_handler(&mut self, handler: InterruptHandler)来设定中断时间
// 模拟器内，使用一个task来模拟中断时间

pub trait TimeSource {
    /// 获取当前时间（带时区）
    async fn get_time(&self) -> Result<Timestamp>;

    /// 通过SNTP更新时间
    async fn set_time(&mut self, new_time: Timestamp) -> Result<()>;
}

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
    async fn get_time(&self) -> Result<Zoned> {
        let timestamp_us = self.timestamp_us.load(Ordering::Acquire) as i64;

        let utc = Zoned::new(
            Timestamp::from_microsecond(timestamp_us).unwrap(),
            TimeZone::UTC,
        );

        Ok(utc)
    }

    async fn update_time_by_sntp(&mut self) -> Result<()> {
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

/// ESP32 RTC时间源 - 使用硬件RTC
#[cfg(feature = "embedded_esp")]
pub struct RtcTimeSource {
    // ESP32 RTC实例
    rtc: Rtc<'static>,
    // 是否已同步（默认false，通过外部接口更新时间后后设为true）
    synchronized: bool,
}

#[cfg(feature = "embedded_esp")]
impl RtcTimeSource {
    pub fn new(lpwr: LPWR<'static>) -> Self {
        log::info!("Initializing RtcTimeSource with hardware RTC");

        let rtc = Rtc::new(lpwr);

        Self {
            rtc,
            synchronized: false,
        }
    }
}

#[cfg(feature = "embedded_esp")]
impl TimeSource for RtcTimeSource {
    async fn get_time(&self) -> Result<Timestamp> {
        let timestamp_us = self.rtc.current_time_us();
        let timestamp =
            Timestamp::from_microsecond(timestamp_us as i64).map_err(|_| AppError::TimeError)?;
        log::debug!("Current RTC time: {}", timestamp);
        Ok(timestamp)
    }

    async fn set_time(&mut self, new_time: Timestamp) -> Result<()> {
        let timestamp_us = new_time.as_microsecond();
        log::debug!("Setting RTC time to: {}", timestamp_us);
        self.rtc.set_current_time_us(timestamp_us as u64);
        Ok(())
    }
}

// 默认时间源选择
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultTimeSource = SimulatedRtc;

#[cfg(feature = "embedded_esp")]
pub type DefaultTimeSource = RtcTimeSource;
