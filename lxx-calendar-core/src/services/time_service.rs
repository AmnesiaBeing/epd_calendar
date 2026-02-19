use lxx_calendar_common::*;
use sxtwl_rs::solar::SolarDay;

pub struct TimeService<R: Rtc> {
    initialized: bool,
    current_time: Option<DateTime>,
    cached_lunar: Option<LunarDate>,
    cached_solar_term: Option<SolarTerm>,
    cached_holiday: Option<Holiday>,
    last_calculation_date: Option<(u16, u8, u8)>,
    timezone_offset: i32,
    rtc: Option<R>,
}

impl<R: Rtc> TimeService<R> {
    pub fn new() -> Self {
        Self {
            initialized: false,
            current_time: None,
            cached_lunar: None,
            cached_solar_term: None,
            cached_holiday: None,
            last_calculation_date: None,
            timezone_offset: 28800,
            rtc: None,
        }
    }

    pub fn with_rtc(rtc: R) -> Self {
        Self {
            initialized: false,
            current_time: None,
            cached_lunar: None,
            cached_solar_term: None,
            cached_holiday: None,
            last_calculation_date: None,
            timezone_offset: 28800,
            rtc: Some(rtc),
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        let current_timestamp = if let Some(ref mut rtc) = self.rtc {
            match rtc.initialize().await {
                Ok(_) => {
                    match rtc.get_time().await {
                        Ok(ts) => {
                            info!("Time initialized from RTC: {}", ts);
                            ts
                        }
                        Err(_) => {
                            let secs_since_boot = embassy_time::Instant::now().elapsed().as_secs();
                            let base_timestamp: i64 = 1704067200;
                            base_timestamp + secs_since_boot as i64
                        }
                    }
                }
                Err(_) => {
                    let secs_since_boot = embassy_time::Instant::now().elapsed().as_secs();
                    let base_timestamp: i64 = 1704067200;
                    base_timestamp + secs_since_boot as i64
                }
            }
        } else {
            let secs_since_boot = embassy_time::Instant::now().elapsed().as_secs();
            let base_timestamp: i64 = 1704067200;
            base_timestamp + secs_since_boot as i64
        };

        self.current_time = Some(self.timestamp_to_datetime(current_timestamp));
        self.timezone_offset = 28800;
        self.initialized = true;

        Ok(())
    }

    pub async fn get_current_time(&self) -> SystemResult<DateTime> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if let Some(ref base_time) = self.current_time {
            let elapsed = embassy_time::Instant::now().elapsed().as_secs();
            let base_timestamp = self.datetime_to_timestamp(base_time);
            Ok(self.timestamp_to_datetime(base_timestamp + elapsed as i64))
        } else {
            Err(SystemError::HardwareError(HardwareError::NotInitialized))
        }
    }

    pub async fn set_time(&mut self, datetime: DateTime) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        self.timezone_offset = datetime.timezone_offset;
        self.current_time = Some(datetime);
        self.invalidate_cache();

        if let Some(ref mut rtc) = self.rtc {
            let timestamp = self.datetime_to_timestamp(&datetime);
            if let Err(e) = rtc.set_time(timestamp).await {
                warn!("Failed to write time to RTC: {:?}", e);
            }
        }

        Ok(())
    }

    pub async fn get_lunar_date(&self) -> SystemResult<LunarDate> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let current_time = self.get_current_time().await?;

        if let Some(ref cached) = self.cached_lunar {
            if let Some(ref last_date) = self.last_calculation_date {
                if last_date.0 == current_time.year
                    && last_date.1 == current_time.month
                    && last_date.2 == current_time.day
                {
                    return Ok(*cached);
                }
            }
        }

        let lunar =
            self.calculate_lunar_date(current_time.year, current_time.month, current_time.day);

        Ok(lunar)
    }

    fn calculate_lunar_date(&self, year: u16, month: u8, day: u8) -> LunarDate {
        let solar_day = SolarDay::from_ymd(year as isize, month as usize, day as usize);
        let lunar_day = solar_day.get_lunar_day();
        let sixty_cycle = solar_day.get_sixty_cycle_day();

        // 获取生肖：年柱的地支 -> 生肖
        let zodiac_idx = sixty_cycle
            .get_year()
            .get_earth_branch()
            .get_zodiac()
            .get_index();

        LunarDate::from_sxtwl(
            lunar_day.get_year() as u16,
            lunar_day.get_month().abs() as u8,
            lunar_day.get_day() as u8,
            lunar_day.get_month() < 0,
            zodiac_idx,
            sixty_cycle.get_year().get_index(),
            sixty_cycle.get_month().get_index(),
            sixty_cycle.get_sixty_cycle().get_index(),
        )
    }

    pub async fn get_solar_term(&self) -> SystemResult<Option<SolarTerm>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let current_time = self.get_current_time().await?;
        Ok(self.calculate_solar_term(current_time.year, current_time.month, current_time.day))
    }

    fn calculate_solar_term(&self, year: u16, month: u8, day: u8) -> Option<SolarTerm> {
        let solar_day = SolarDay::from_ymd(year as isize, month as usize, day as usize);
        let term = solar_day.get_term();
        SolarTerm::from_index(term.get_index(), day, month)
    }

    pub async fn get_holiday(&self) -> SystemResult<Option<Holiday>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let current_time = self.get_current_time().await?;
        Ok(self.calculate_holiday(current_time.month, current_time.day))
    }

    fn calculate_holiday(&self, month: u8, day: u8) -> Option<Holiday> {
        match (month, day) {
            (1, 1) => Some(Holiday::NewYear),
            (5, 1) => Some(Holiday::LaborDay),
            (10, 1) => Some(Holiday::NationalDay),
            _ => None,
        }
    }

    pub async fn calculate_wakeup_schedule(&self) -> SystemResult<WakeupSchedule> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let current_time = self.get_current_time().await?;
        let next_wakeup =
            self.datetime_to_timestamp(&current_time) + ((60 - current_time.second) as i64);

        Ok(WakeupSchedule {
            next_wakeup_time: next_wakeup,
            wakeup_reason: WakeupReason::Timer,
            scheduled_tasks: ScheduledTasks {
                display_refresh: current_time.second == 0,
                network_sync: current_time.minute == 0,
                alarm_check: true,
                reserved: 0,
            },
        })
    }

    pub async fn calculate_next_wakeup_time(&self) -> SystemResult<u64> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let current_time = self.get_current_time().await?;
        Ok(embassy_time::Instant::now().elapsed().as_secs() + (60 - current_time.second) as u64)
    }

    fn invalidate_cache(&mut self) {
        self.cached_lunar = None;
        self.cached_solar_term = None;
        self.cached_holiday = None;
        self.last_calculation_date = None;
    }

    fn is_leap_year(year: u16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    fn days_in_month(year: u16, month: u8) -> u8 {
        const DAYS: [[u8; 12]; 2] = [
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
        ];
        DAYS[Self::is_leap_year(year) as usize][month as usize - 1]
    }

    fn datetime_to_timestamp(&self, dt: &DateTime) -> i64 {
        let mut ts = 0i64;

        for y in 1970..dt.year {
            ts += if Self::is_leap_year(y) { 366 } else { 365 } * 86400;
        }

        for m in 1..dt.month {
            ts += Self::days_in_month(dt.year, m) as i64 * 86400;
        }

        ts += (dt.day as i64 - 1) * 86400;
        ts += dt.hour as i64 * 3600;
        ts += dt.minute as i64 * 60;
        ts += dt.second as i64;
        ts -= self.timezone_offset as i64;

        ts
    }

    fn timestamp_to_datetime(&self, ts: i64) -> DateTime {
        let mut ts = ts + self.timezone_offset as i64;

        let mut year = 1970u16;
        loop {
            let days = if Self::is_leap_year(year) { 366 } else { 365 };
            if ts >= days as i64 * 86400 {
                ts -= days as i64 * 86400;
                year += 1;
            } else {
                break;
            }
        }

        let mut month = 1u8;
        for m in 1..=12 {
            let days = Self::days_in_month(year, m) as i64 * 86400;
            if ts >= days {
                ts -= days;
                month += 1;
            } else {
                break;
            }
        }

        let day = (ts / 86400) as u8 + 1;
        ts = ts % 86400;

        let hour = (ts / 3600) as u8;
        ts = ts % 3600;

        let minute = (ts / 60) as u8;
        let second = (ts % 60) as u8;

        let weekday = Self::weekday_from_ymd(year, month, day);

        DateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
            weekday,
            timezone_offset: self.timezone_offset,
        }
    }

    fn weekday_from_ymd(year: u16, month: u8, day: u8) -> u8 {
        let solar_day = SolarDay::from_ymd(year as isize, month as usize, day as usize);
        solar_day.get_week().get_index() as u8
    }
}
