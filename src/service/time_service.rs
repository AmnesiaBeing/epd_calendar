// src/service/time_service.rs

//! 时间服务模块 - 提供系统时间获取和格式化功能
//! 
//! 该模块从时间源驱动获取当前时间，并转换为适合显示的格式。

use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::common::system_state::TimeData;
use crate::driver::time_source::{DefaultTimeSource, TimeSource};

/// 时间服务，提供系统时间获取和格式化功能
pub struct TimeService {
    /// 时间源驱动实例（全局互斥锁保护）
    time_source: &'static GlobalMutex<DefaultTimeSource>,
}

impl TimeService {
    /// 创建新的时间服务实例
    /// 
    /// # 参数
    /// - `time_source`: 时间源驱动实例
    /// 
    /// # 返回值
    /// 返回新的TimeService实例
    pub fn new(time_source: &'static GlobalMutex<DefaultTimeSource>) -> Self {
        Self { time_source }
    }

    /// 获取当前时间数据
    /// 
    /// # 返回值
    /// - `Result<TimeData>`: 成功返回时间数据，失败返回错误
    pub async fn get_current_time(&self) -> Result<TimeData> {
        let datetime = self
            .time_source
            .lock()
            .await
            .get_time()
            .map_err(|_| AppError::TimeError)?;

        let zoned = datetime.to_zoned(TimeZone::fixed(Offset::constant(8)));

        let datetime: DateTime = zoned.into();

        Ok(TimeData {
            hour: datetime.hour() as u8,
            minute: datetime.minute() as u8,
            am_pm: None,
        })
    }
}