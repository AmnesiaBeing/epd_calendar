use alloc::vec::Vec;
use embassy_time::{Duration, Instant};
use lxx_calendar_common::*;
use sxtwl_rs::festival::{LunarFestival, SolarFestival};
use sxtwl_rs::solar::SolarDay;

pub struct TimeService<R: Rtc> {
    initialized: bool,
    boot_instant: Option<Instant>,
    cached_solar_time: Option<SolarTime>,
    cached_weekday: Option<Week>,
    cached_lunar: Option<LunarDay>,
    cached_solar_term: Option<SolarTerm>,
    cached_solar_festival: Option<SolarFestival>,
    cached_lunar_festival: Option<LunarFestival>,
    last_calculation_date: Option<(u16, u8, u8, u8)>,
    timezone_offset: i32,
    rtc: Option<R>,
}

impl<R: Rtc> TimeService<R> {
    pub fn new() -> Self {
        Self {
            initialized: false,
            boot_instant: None,
            cached_solar_time: None,
            cached_weekday: None,
            cached_lunar: None,
            cached_solar_term: None,
            cached_solar_festival: None,
            cached_lunar_festival: None,
            last_calculation_date: None,
            timezone_offset: 28800,
            rtc: None,
        }
    }

    pub fn with_rtc(mut self, rtc: R) -> Self {
        self.rtc = Some(rtc);
        self
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        self.boot_instant = Some(Instant::now());

        if let Some(ref mut rtc) = self.rtc {
            let timestamp = rtc.get_time().await.unwrap_or(1704067200);
            let (solar_time, weekday) = self.timestamp_to_time_components(timestamp);
            self.timezone_offset = 28800;
            self.cached_solar_time = Some(solar_time);
            self.cached_weekday = Some(weekday);
        }

        self.initialized = true;

        Ok(())
    }

    pub async fn get_solar_time(&mut self) -> SystemResult<SolarTime> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if let Some(ref cached) = self.cached_solar_time {
            return Ok(*cached);
        }

        if let Some(ref mut rtc) = self.rtc {
            let timestamp = rtc.get_time().await.unwrap_or(1704067200);
            let (solar_time, _) = self.timestamp_to_time_components(timestamp);
            self.cached_solar_time = Some(solar_time);
            Ok(solar_time)
        } else {
            Err(SystemError::HardwareError(HardwareError::NotInitialized))
        }
    }

    pub async fn get_weekday(&mut self) -> SystemResult<Week> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if let Some(ref cached) = self.cached_weekday {
            return Ok(cached.clone());
        }

        if let Some(ref cached) = self.cached_solar_time {
            let weekday = cached.get_julian_day().get_solar_day().get_week();
            self.cached_weekday = Some(weekday.clone());
            return Ok(weekday);
        }

        let solar_time = self.get_solar_time().await?;
        let weekday = solar_time.get_julian_day().get_solar_day().get_week();
        self.cached_weekday = Some(weekday.clone());
        Ok(weekday)
    }

    fn timestamp_to_time_components(&self, timestamp: i64) -> (SolarTime, Week) {
        let ts = timestamp;

        let mut year = 1970i16;
        let mut remaining_ts = ts;
        loop {
            let days = if Self::is_leap_year(year) { 366 } else { 365 };
            if remaining_ts >= days as i64 * 86400 {
                remaining_ts -= days as i64 * 86400;
                year += 1;
            } else {
                break;
            }
        }

        let mut month = 1i8;
        loop {
            let days = Self::days_in_month(year, month) as i64 * 86400;
            if remaining_ts >= days {
                remaining_ts -= days;
                month += 1;
            } else {
                break;
            }
        }

        let day = (remaining_ts / 86400) as u8 + 1;
        remaining_ts = remaining_ts % 86400;

        let hour = (remaining_ts / 3600) as u8;
        remaining_ts = remaining_ts % 3600;

        let minute = (remaining_ts / 60) as u8;
        let second = (remaining_ts % 60) as u8;

        let timezone_hours = self.timezone_offset as i32 / 3600;
        let mut local_year = year;
        let mut local_month = month as i32;
        let mut local_day = day as i32;
        let mut local_hour = hour as i32 + timezone_hours;

        if local_hour < 0 {
            local_hour += 24;
            local_day -= 1;
            if local_day < 1 {
                local_month -= 1;
                if local_month < 1 {
                    local_month = 12;
                    local_year -= 1;
                }
                local_day = Self::days_in_month(local_year, local_month as i8) as i32;
            }
        } else if local_hour >= 24 {
            local_hour -= 24;
            local_day += 1;
            let days_in_current_month = Self::days_in_month(local_year, local_month as i8) as i32;
            if local_day > days_in_current_month {
                local_day = 1;
                local_month += 1;
                if local_month > 12 {
                    local_month = 1;
                    local_year += 1;
                }
            }
        }

        let solar_time = SolarTime::from_ymd_hms(
            local_year as isize,
            local_month as usize,
            local_day as usize,
            local_hour as usize,
            minute as usize,
            second as usize,
        );

        let weekday = solar_time.get_julian_day().get_solar_day().get_week();

        (solar_time, weekday)
    }

    pub async fn get_lunar_date(&mut self) -> SystemResult<LunarDay> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let solar_time = self.get_solar_time().await?;
        let year = solar_time.get_year() as u16;
        let month = solar_time.get_month() as u8;
        let day = solar_time.get_day() as u8;

        if let Some(ref cached) = self.cached_lunar {
            if let Some(ref last_date) = self.last_calculation_date {
                if last_date.0 == year
                    && last_date.1 == month
                    && last_date.2 == day
                    && last_date.3 == 0
                {
                    return Ok(cached.clone());
                }
            }
        }

        let lunar_day = self.calculate_lunar_date(year, month, day);

        self.cached_lunar = Some(lunar_day.clone());
        self.last_calculation_date = Some((year, month, day, 0));

        Ok(lunar_day)
    }

    fn calculate_lunar_date(&self, year: u16, month: u8, day: u8) -> LunarDay {
        let solar_day = SolarDay::from_ymd(year as isize, month as usize, day as usize);
        solar_day.get_lunar_day()
    }

    pub async fn get_solar_term(&mut self) -> SystemResult<Option<SolarTerm>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let solar_time = self.get_solar_time().await?;
        let year = solar_time.get_year() as u16;
        let month = solar_time.get_month() as u8;
        let day = solar_time.get_day() as u8;

        if let Some(ref cached) = self.cached_solar_term {
            if let Some(ref last_date) = self.last_calculation_date {
                if last_date.0 == year
                    && last_date.1 == month
                    && last_date.2 == day
                    && last_date.3 == 1
                {
                    return Ok(Some(cached.clone()));
                }
            }
        }

        let term = self.calculate_solar_term(year, month, day);
        self.cached_solar_term = term.clone();
        self.last_calculation_date = Some((year, month, day, 1));

        Ok(term)
    }

    fn calculate_solar_term(&self, year: u16, month: u8, day: u8) -> Option<SolarTerm> {
        let solar_day = SolarDay::from_ymd(year as isize, month as usize, day as usize);
        let term = solar_day.get_term();
        let index = term.get_index() as isize;
        if index == 0 {
            return None;
        }
        Some(SolarTerm::from_index(year as isize, index))
    }

    pub async fn get_solar_festival(&mut self) -> SystemResult<Option<SolarFestival>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let solar_time = self.get_solar_time().await?;
        let year = solar_time.get_year() as u16;
        let month = solar_time.get_month() as u8;
        let day = solar_time.get_day() as u8;

        if let Some(ref cached) = self.cached_solar_festival {
            if let Some(ref last_date) = self.last_calculation_date {
                if last_date.0 == year
                    && last_date.1 == month
                    && last_date.2 == day
                    && last_date.3 == 2
                {
                    return Ok(Some(cached.clone()));
                }
            }
        }

        let festival = self.calculate_solar_festival(year, month, day);
        self.cached_solar_festival = festival.clone();
        self.last_calculation_date = Some((year, month, day, 2));

        Ok(festival)
    }

    fn calculate_solar_festival(&self, year: u16, month: u8, day: u8) -> Option<SolarFestival> {
        let solar_day = SolarDay::from_ymd(year as isize, month as usize, day as usize);
        solar_day.get_festival()
    }

    pub async fn get_lunar_festival(&mut self) -> SystemResult<Option<LunarFestival>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let solar_time = self.get_solar_time().await?;
        let year = solar_time.get_year() as u16;
        let month = solar_time.get_month() as u8;
        let day = solar_time.get_day() as u8;

        if let Some(ref cached) = self.cached_lunar_festival {
            if let Some(ref last_date) = self.last_calculation_date {
                if last_date.0 == year
                    && last_date.1 == month
                    && last_date.2 == day
                    && last_date.3 == 3
                {
                    return Ok(Some(cached.clone()));
                }
            }
        }

        let festival = self.calculate_lunar_festival(year, month, day);
        self.cached_lunar_festival = festival.clone();
        self.last_calculation_date = Some((year, month, day, 3));

        Ok(festival)
    }

    fn calculate_lunar_festival(&self, year: u16, month: u8, day: u8) -> Option<LunarFestival> {
        let solar_day = SolarDay::from_ymd(year as isize, month as usize, day as usize);
        let lunar_day = solar_day.get_lunar_day();
        lunar_day.get_festival()
    }

    fn is_leap_year(year: i16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    fn days_in_month(year: i16, month: i8) -> u8 {
        const DAYS: [[u8; 12]; 2] = [
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
        ];
        DAYS[Self::is_leap_year(year) as usize][month as usize - 1]
    }

    fn solar_time_to_timestamp(&self, st: &SolarTime) -> i64 {
        let mut ts = 0i64;
        let year = st.get_year() as i16;

        for y in 1970..=year {
            ts += if Self::is_leap_year(y) { 366 } else { 365 } * 86400;
        }

        for m in 1..st.get_month() as i8 {
            ts += Self::days_in_month(year, m) as i64 * 86400;
        }

        ts += (st.get_day() as i64 - 1) * 86400;
        ts += st.get_hour() as i64 * 3600;
        ts += st.get_minute() as i64 * 60;
        ts += st.get_second() as i64;
        ts -= self.timezone_offset as i64;

        ts
    }

    pub async fn get_next_hour_chime_time(&mut self, enabled: bool) -> SystemResult<Option<u64>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        if !enabled {
            return Ok(None);
        }

        let solar_time = self.get_solar_time().await?;
        let current_minute = solar_time.get_minute() as u8;
        let current_second = solar_time.get_second() as u8;

        let current_timestamp = self.solar_time_to_timestamp(&solar_time) as u64;
        let mut next_timestamp = current_timestamp;

        if current_minute == 59 && current_second >= 55 {
        } else if current_minute < 59 {
            next_timestamp += (59 - current_minute as u64 - 1) * 60;
            next_timestamp += 60 - current_second as u64;
        } else {
            next_timestamp += (24 * 3600) - (current_minute as u64 * 60) - current_second as u64;
            next_timestamp += 55 * 60;
        }

        Ok(Some(next_timestamp))
    }

    pub async fn get_next_alarm_time(&mut self, alarms: &[AlarmInfo]) -> SystemResult<Option<u64>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let solar_time = self.get_solar_time().await?;
        let current_timestamp = self.solar_time_to_timestamp(&solar_time) as u64;
        let current_hour = solar_time.get_hour() as u8;
        let current_minute = solar_time.get_minute() as u8;
        let current_second = solar_time.get_second() as u8;

        let mut nearest_alarm_timestamp: Option<u64> = None;

        for alarm in alarms {
            if !alarm.enabled {
                continue;
            }

            let mut alarm_timestamp = current_timestamp;

            let (hour_diff, minute_diff) = if alarm.hour > current_hour {
                (
                    alarm.hour - current_hour,
                    alarm.minute as i16 - current_minute as i16,
                )
            } else if alarm.hour < current_hour {
                (
                    24 - current_hour + alarm.hour,
                    alarm.minute as i16 - current_minute as i16,
                )
            } else {
                if alarm.minute > current_minute {
                    (0, alarm.minute as i16 - current_minute as i16)
                } else {
                    (24, alarm.minute as i16 - current_minute as i16 + 60)
                }
            };

            let total_seconds =
                (hour_diff as i64 * 3600) + (minute_diff as i64 * 60) - current_second as i64;
            alarm_timestamp += total_seconds as u64;

            if nearest_alarm_timestamp.is_none()
                || alarm_timestamp < nearest_alarm_timestamp.unwrap()
            {
                nearest_alarm_timestamp = Some(alarm_timestamp);
            }
        }

        Ok(nearest_alarm_timestamp)
    }

    pub async fn get_next_display_refresh_time(
        &mut self,
        refresh_interval_minutes: u8,
    ) -> SystemResult<Option<u64>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let solar_time = self.get_solar_time().await?;
        let current_timestamp = self.solar_time_to_timestamp(&solar_time) as u64;
        let current_minute = solar_time.get_minute() as u8;
        let current_second = solar_time.get_second() as u8;

        let next_refresh_minute =
            ((current_minute / refresh_interval_minutes + 1) * refresh_interval_minutes) % 60;
        let mut next_refresh_timestamp = current_timestamp + (60 - current_second as u64) % 60;

        if next_refresh_minute > current_minute {
            next_refresh_timestamp += (next_refresh_minute - current_minute) as u64 * 60;
        } else {
            next_refresh_timestamp +=
                (60 - current_minute as u64 + next_refresh_minute as u64) * 60;
        }

        Ok(Some(next_refresh_timestamp))
    }

    pub async fn calculate_next_wakeup_time(
        &mut self,
        config: &SystemConfig,
    ) -> SystemResult<Option<(u64, WakeupSource)>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let solar_time = self.get_solar_time().await?;

        let mut candidates: Vec<(u64, WakeupSource)> = Vec::new();

        if let Some(ts) = self
            .get_next_hour_chime_time(config.time_config.hour_chime_enabled)
            .await?
        {
            candidates.push((
                ts,
                WakeupSource::HourChime(((solar_time.get_hour() as u8) + 1) % 24),
            ));
        }

        if let Some(ts) = self.get_next_alarm_time(&config.time_config.alarms).await? {
            candidates.push((ts, WakeupSource::Alarm));
        }

        if let Some(ts) = self
            .get_next_display_refresh_time(
                (config.display_config.refresh_interval_seconds / 60) as u8,
            )
            .await?
        {
            candidates.push((ts, WakeupSource::DisplayRefresh));
        }

        if let Some(ts) = self.get_next_network_sync_time().await? {
            candidates.push((ts, WakeupSource::NetworkSync));
        }

        if candidates.is_empty() {
            return Ok(None);
        }

        candidates.sort_by_key(|(ts, _)| *ts);

        let min_wakeup = candidates.first().map(|(ts, source)| (*ts, source.clone()));

        Ok(min_wakeup)
    }

    async fn get_next_network_sync_time(&mut self) -> SystemResult<Option<u64>> {
        Ok(None)
    }

    pub async fn get_timestamp(&self) -> SystemResult<u64> {
        if let Some(ref rtc) = self.rtc {
            let time = rtc.get_time().await.unwrap_or(1704067200);
            Ok(time as u64)
        } else {
            Err(SystemError::HardwareError(HardwareError::NotInitialized))
        }
    }

    pub async fn set_rtc_alarm(&mut self, timestamp: u64) -> SystemResult<()> {
        if let Some(ref mut rtc) = self.rtc {
            let current_time = rtc.get_time().await.unwrap_or(1704067200) as u64;
            if timestamp > current_time {
                let duration = Duration::from_millis((timestamp - current_time) * 1000);
                rtc.set_wakeup(duration).await.ok();
                info!(
                    "RTC alarm set for {} seconds later",
                    timestamp - current_time
                );
            }
        }
        Ok(())
    }

    pub async fn enter_light_sleep(&mut self) {
        if let Some(ref mut rtc) = self.rtc {
            rtc.sleep_light().await;
        }
    }

    pub async fn set_time(&mut self, timestamp: u64) -> SystemResult<()> {
        if let Some(ref mut rtc) = self.rtc {
            rtc.set_time(timestamp as i64).await.ok();
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeupSource {
    HourChime(u8),
    Alarm,
    DisplayRefresh,
    NetworkSync,
}
