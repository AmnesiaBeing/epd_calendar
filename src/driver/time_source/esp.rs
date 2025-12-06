// src/driver/time_source/esp.rs

//! ESP32平台时间源驱动实现
//! 
//! 提供ESP32平台的硬件RTC时间功能，基于esp-hal库实现

#[cfg(feature = "embedded_esp")]
use esp_hal::peripherals::Peripherals;
use esp_hal::rtc_cntl::Rtc;
use jiff::Timestamp;

use crate::common::error::{AppError, Result};
use crate::driver::time_source::TimeSource;

/// ESP32 RTC时间源结构体
/// 
/// 使用ESP32硬件RTC提供系统时间功能
#[cfg(feature = "embedded_esp")]
pub struct RtcTimeSource {
    /// ESP32 RTC实例
    rtc: Rtc<'static>,
    /// 是否已同步（通过外部接口更新时间后设为true）
    synchronized: bool,
}

#[cfg(feature = "embedded_esp")]
impl RtcTimeSource {
    /// 创建新的ESP32 RTC时间源实例
    /// 
    /// # 参数
    /// - `peripherals`: ESP32硬件外设
    /// 
    /// # 返回值
    /// - `Self`: 时间源实例
    pub fn new(peripherals: &Peripherals) -> Self {
        log::info!("Initializing RtcTimeSource with hardware RTC");

        let rtc = Rtc::new(unsafe { peripherals.LPWR.clone_unchecked() });

        Self {
            rtc,
            synchronized: false,
        }
    }
}

#[cfg(feature = "embedded_esp")]
impl TimeSource for RtcTimeSource {
    /// 获取当前时间
    /// 
    /// # 返回值
    /// - `Result<Timestamp>`: 当前时间戳或错误
    fn get_time(&self) -> Result<Timestamp> {
        let timestamp_us = self.rtc.current_time_us();
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
        self.rtc.set_current_time_us(timestamp_us as u64);
        self.synchronized = true;
        Ok(())
    }
}