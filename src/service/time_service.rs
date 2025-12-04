// src/service/time_service.rs
use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};

use crate::common::error::{AppError, Result};
use crate::common::system_state::TimeData;
use crate::driver::ntp_source::SntpSource;
use crate::driver::time_source::{DefaultTimeSource, TimeSource};

pub struct TimeService {
    time_source: DefaultTimeSource,
    ntp_source: SntpSource,
}

impl TimeService {
    pub fn new(time_source: DefaultTimeSource, ntp_source: SntpSource) -> Self {
        Self {
            time_source,
            ntp_source: ntp_source,
        }
    }

    pub async fn update_time_by_sntp(&mut self) -> Result<()> {
        // TODO: 增加平滑修改时间的逻辑（需要修改TimeSource）
        let sntp_time = self.ntp_source.sync_time().await?;
        log::info!("NTP time received: {}", sntp_time);

        self.time_source.set_time(sntp_time)?;

        Ok(())
    }

    pub fn get_current_time(&self) -> Result<TimeData> {
        let datetime = self
            .time_source
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
