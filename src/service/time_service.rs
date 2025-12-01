use jiff::civil::DateTime;

// src/service/time_service.rs
use crate::common::error::{AppError, Result};
use crate::common::types::TimeData;
use crate::driver::time_source::{DefaultTimeSource, TimeSource};

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
        // self.time_source.set_time().await?;
        Ok(())
    }

    pub async fn get_current_time(&self) -> Result<TimeData> {
        let datetime = self
            .time_source
            .get_time()
            .await
            .map_err(|_| AppError::TimeError)?;

        let zoned = datetime.in_tz("Asia/Shanghai").unwrap();

        let datetime: DateTime = zoned.into();

        Ok(TimeData {
            hour: datetime.hour() as u8,
            minute: datetime.minute() as u8,
            is_24_hour: self.is_24_hour,
        })
    }

    pub fn set_24_hour_format(&mut self, enabled: bool) {
        self.is_24_hour = enabled;
    }

    pub fn set_temperature_celsius(&mut self, enabled: bool) {
        self.temperature_celsius = enabled;
    }
}
