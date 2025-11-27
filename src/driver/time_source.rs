// src/driver/time_source.rs
use crate::common::error::{AppError, Result};
use chrono::{DateTime, Local};

pub trait TimeSource {
    /// 获取当前时间
    async fn now(&self) -> Result<DateTime<Local>>;

    /// 检查RTC是否可用
    fn is_rtc_available(&self) -> bool;

    /// 同步到RTC（如果支持）
    async fn sync_to_rtc(&mut self, datetime: DateTime<Local>) -> Result<()>;
}

/// 系统时间源（用于模拟器和Linux）
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct SystemTimeSource;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub use SystemTimeSource as DefaultTimeSource;

impl SystemTimeSource {
    pub fn new() -> Self {
        Self
    }
}

impl TimeSource for SystemTimeSource {
    async fn now(&self) -> Result<DateTime<Local>> {
        Ok(Local::now())
    }

    fn is_rtc_available(&self) -> bool {
        false
    }

    async fn sync_to_rtc(&mut self, _datetime: DateTime<Local>) -> Result<()> {
        Err(AppError::TimeError) // 系统时间源不支持RTC同步
    }
}

/// NTP时间源
pub struct NtpTimeSource {
    server: String,
    last_sync: Option<DateTime<Local>>,
}

impl NtpTimeSource {
    pub fn new(server: &str) -> Self {
        Self {
            server: server.to_string(),
            last_sync: None,
        }
    }

    pub async fn sync_with_ntp(&mut self) -> Result<()> {
        // 使用sntpc库进行NTP同步
        // 这里简化实现，返回系统时间
        self.last_sync = Some(Local::now());
        Ok(())
    }
}

impl TimeSource for NtpTimeSource {
    async fn now(&self) -> Result<DateTime<Local>> {
        Ok(Local::now())
    }

    fn is_rtc_available(&self) -> bool {
        false
    }

    async fn sync_to_rtc(&mut self, _datetime: DateTime<Local>) -> Result<()> {
        Err(AppError::TimeError)
    }
}

// 为嵌入式系统准备的RTC时间源
#[cfg(feature = "embedded_esp")]
pub struct RtcTimeSource {
    // RTC硬件驱动
}

#[cfg(feature = "embedded_esp")]
impl RtcTimeSource {
    pub fn new() -> Result<Self> {
        // 初始化RTC硬件
        Ok(Self {})
    }
}

#[cfg(feature = "embedded_esp")]
impl TimeSource for RtcTimeSource {
    async fn now(&self) -> Result<DateTime<Local>> {
        // 从RTC硬件读取时间
        // 简化实现
        Ok(Local::now())
    }

    fn is_rtc_available(&self) -> bool {
        true
    }

    async fn sync_to_rtc(&mut self, datetime: DateTime<Local>) -> Result<()> {
        // 将时间写入RTC硬件
        Ok(())
    }
}
