// src/kernel/driver/time_source/linux.rs

//! Linux平台时间源驱动实现
//!
//! 提供Linux平台的模拟RTC时间功能，使用原子操作实现线程安全的时间管理

use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

use jiff::Timestamp;

use crate::common::error::{AppError, Result};
use crate::kernel::driver::time_driver::TimeDriver;

/// 模拟器RTC时间源结构体
///
/// 模拟ESP32的RTC行为，使用原子操作实现线程安全的时间管理
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct SimulatedRtc {
    /// 模拟ESP32 RTC的64位微秒时间戳
    timestamp_us: AtomicU64,
    /// 是否已同步（通过外部接口更新时间后设为true）
    synchronized: bool,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl SimulatedRtc {
    /// 创建新的模拟RTC时间源实例
    ///
    /// # 返回值
    /// - `Self`: 时间源实例
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
impl TimeDriver for SimulatedRtc {
    /// 获取当前时间
    ///
    /// # 返回值
    /// - `Result<Timestamp>`: 当前时间戳或错误
    fn get_time(&self) -> Result<Timestamp> {
        let timestamp_us = self.timestamp_us.load(Ordering::Acquire) as i64;

        let timestamp =
            Timestamp::from_microsecond(timestamp_us as i64).map_err(|_| AppError::TimeError)?;
        log::debug!("Current RTC time: {}", timestamp);
        Ok(timestamp)
    }

    /// 设置新时间
    ///
    /// # 参数
    /// - `new_time`: 新的时间戳
    ///
    /// # 返回值
    /// - `Result<()>`: 设置结果
    fn set_time(&mut self, new_time: Timestamp) -> Result<()> {
        let timestamp_us = new_time.as_microsecond();
        log::debug!("Setting RTC time to: {}", timestamp_us);
        self.timestamp_us
            .store(timestamp_us as u64, Ordering::Release);
        self.synchronized = true;
        Ok(())
    }
}
