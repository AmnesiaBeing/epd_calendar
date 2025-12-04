// src/service/time_service.rs
use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::common::system_state::TimeData;
use crate::driver::time_source::{DefaultTimeSource, TimeSource};

pub struct TimeService {
    time_source: &'static GlobalMutex<DefaultTimeSource>,
}

impl TimeService {
    pub fn new(time_source: &'static GlobalMutex<DefaultTimeSource>) -> Self {
        Self { time_source }
    }

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
