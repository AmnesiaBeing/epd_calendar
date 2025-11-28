// src/service/time_service.rs
use crate::common::error::{AppError, Result};
use crate::common::types::TimeData;
use crate::driver::time_source::{DefaultTimeSource, TimeSource};
use chrono::{DateTime, Datelike, Local, Timelike};

pub struct TimeService {
    time_source: DefaultTimeSource,
    is_24_hour: bool,
    temperature_celsius: bool,
}

impl TimeService {
    pub fn new(
        time_source: DefaultTimeSource,
        is_24_hour: bool,
        temperature_celsius: bool,
    ) -> Self {
        Self {
            time_source,
            is_24_hour,
            temperature_celsius,
        }
    }

    pub async fn update_time_by_sntp(&mut self) -> Result<()> {
        self.time_source.update_time_by_sntp().await?;
        Ok(())
    }

    pub async fn get_current_time(&self) -> Result<TimeData> {
        let datetime = self
            .time_source
            .get_time()
            .await
            .map_err(|_| AppError::TimeError)?;

        // 获取农历信息
        // let lunar_data = self.calculate_lunar_data(&datetime)?;

        Ok(TimeData {
            hour: datetime.hour() as u8,
            minute: datetime.minute() as u8,
            is_24_hour: self.is_24_hour,
            date_string: self.format_date(&datetime),
            weekday: self.get_weekday_chinese(datetime.weekday()),
            // holiday: self.get_holiday(datetime.year(), datetime.month(), datetime.day()),
            // lunar: lunar_data,
        })
    }

    pub fn set_24_hour_format(&mut self, enabled: bool) {
        self.is_24_hour = enabled;
    }

    pub fn set_temperature_celsius(&mut self, enabled: bool) {
        self.temperature_celsius = enabled;
    }

    fn format_date(&self, datetime: &DateTime<Local>) -> String {
        format!(
            "{:04}-{:02}-{:02}",
            datetime.year(),
            datetime.month(),
            datetime.day()
        )
    }

    fn get_weekday_chinese(&self, weekday: chrono::Weekday) -> String {
        match weekday {
            chrono::Weekday::Mon => "周一".to_string(),
            chrono::Weekday::Tue => "周二".to_string(),
            chrono::Weekday::Wed => "周三".to_string(),
            chrono::Weekday::Thu => "周四".to_string(),
            chrono::Weekday::Fri => "周五".to_string(),
            chrono::Weekday::Sat => "周六".to_string(),
            chrono::Weekday::Sun => "周日".to_string(),
        }
    }
}
