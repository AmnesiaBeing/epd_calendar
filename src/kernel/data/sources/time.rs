//! 时间数据源模块
//! 提供时间和日期相关数据的数据源实现

use alloc::boxed::Box;
use alloc::string::ToString;
use async_trait::async_trait;
use core::str::FromStr;
use embassy_time::{Duration, Instant};
use heapless::String;
use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};
use sxtwl_rs::lunar::LunarDay;
use sxtwl_rs::solar::SolarDay;
use sxtwl_rs::types::{Culture, Tyme};

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::data::types::DynamicValue;
use crate::kernel::data::{DataSource, DataSourceCache};
use crate::kernel::driver::time_driver::{DefaultTimeDriver, TimeDriver};
use crate::kernel::system::api::DefaultSystemApi;

/// 时间数据源结构体
pub struct TimeDataSource {
    /// 时间源驱动实例（全局互斥锁保护）
    time_source: &'static GlobalMutex<DefaultTimeDriver>,
    /// 数据源缓存
    cache: DataSourceCache,
    /// sxtwl 公历日期结构体
    sxtwl_solar_day: Option<SolarDay>,
    /// sxtwl 农历日期结构体
    sxtwl_lunar_day: Option<LunarDay>,
    /// 上一次更新农历的距今的日期
    last_update_lunar: u8,
    /// 新增：当前缓存的公历日期（年、月、日），用于判断是否跨日
    current_date: Option<(isize, usize, usize)>, // (year, month, day)
}

impl TimeDataSource {
    /// 创建新的时间数据源实例
    pub fn new(time_source: &'static GlobalMutex<DefaultTimeDriver>) -> Result<Self> {
        Ok(Self {
            time_source,
            cache: DataSourceCache::default(),
            sxtwl_solar_day: None,
            sxtwl_lunar_day: None,
            last_update_lunar: 0,
            current_date: None, // 初始化为 None，首次刷新时初始化
        })
    }

    /// 初始化/刷新公历&农历数据（基于实际时间）
    fn init_solar_lunar(&mut self, year: isize, month: usize, day: usize) -> Result<()> {
        let solar = SolarDay::new(year, month, day).map_err(|_| {
            log::error!("Invalid solar date: {}-{}-{}", year, month, day);
            AppError::LunarCalculationError
        })?;
        let lunar = solar.get_lunar_day();

        self.sxtwl_solar_day = Some(solar);
        self.sxtwl_lunar_day = Some(lunar);
        self.last_update_lunar = 0; // 重置计数

        Ok(())
    }

    /// 推算下一日的公历&农历数据
    fn next_date(&mut self) -> Result<()> {
        // 确保已有初始化数据
        let Some(mut solar) = self.sxtwl_solar_day.take() else {
            log::error!("Solar data not initialized");
            return Err(AppError::LunarCalculationError);
        };
        let Some(mut lunar) = self.sxtwl_lunar_day.take() else {
            log::error!("Lunar data not initialized");
            return Err(AppError::LunarCalculationError);
        };

        if self.last_update_lunar <= 15 {
            // 直接推算，减少计算量
            solar = solar.next(1);
            lunar = lunar.next(1);
            self.last_update_lunar += 1;
        } else {
            // 超过15天重新计算，防止累计误差
            solar = solar.next(1);
            lunar = solar.get_lunar_day();
            self.last_update_lunar = 0;
        }

        self.sxtwl_solar_day = Some(solar);
        self.sxtwl_lunar_day = Some(lunar);
        Ok(())
    }

    /// 辅助函数：安全设置缓存字段，避免 unwrap panic
    fn set_cache_field(&mut self, name: &str, value: DynamicValue) -> Result<()> {
        let field_name = String::from_str(name).map_err(|_| {
            log::error!("Invalid field name: {}", name);
            AppError::InvalidFieldName
        })?;

        self.cache
            .set_field(field_name, value)
            .map_err(|_| AppError::CacheSetFailed)
    }

    /// 新增：更新日期（公历+农历）字段的核心逻辑（仅跨日时调用）
    fn update_date_fields(
        &mut self,
        year: isize,
        month: usize,
        day: usize,
        weekday: i32,
    ) -> Result<()> {
        // 1. 初始化/更新公历&农历数据
        if self.sxtwl_solar_day.is_none() {
            // 首次刷新：直接初始化
            self.init_solar_lunar(year, month, day)?;
        } else {
            // 跨日：推算下一日数据
            self.next_date()?;
        }

        // 2. 更新公历日期缓存字段
        self.set_cache_field("date.year", DynamicValue::Integer(year as i32))?;
        self.set_cache_field("date.month", DynamicValue::Integer(month as i32))?;
        self.set_cache_field("date.day", DynamicValue::Integer(day as i32))?;
        self.set_cache_field("date.week", DynamicValue::Integer(weekday))?;

        // 3. 提取农历数据并更新缓存（局部作用域避免借用冲突）
        let (lunar_month, lunar_day_num, ganzhi_str, zodiac_str) = {
            let lunar_day = self
                .sxtwl_lunar_day
                .as_ref()
                .ok_or(AppError::LunarCalculationError)?;

            let month = lunar_day.get_month() as i32;
            let day = lunar_day.get_day() as i32;

            let ganzhi = lunar_day
                .get_lunar_month()
                .get_lunar_year()
                .get_sixty_cycle()
                .get_name();
            let ganzhi_str =
                String::from_str(&ganzhi).map_err(|_| AppError::LunarCalculationError)?;

            let zodiac = lunar_day
                .get_lunar_month()
                .get_lunar_year()
                .get_sixty_cycle()
                .get_earth_branch()
                .get_zodiac()
                .get_name();
            let zodiac_str =
                String::from_str(&zodiac).map_err(|_| AppError::LunarCalculationError)?;

            (month, day, ganzhi_str, zodiac_str)
        };

        self.set_cache_field("lunar.month", DynamicValue::Integer(lunar_month))?;
        self.set_cache_field("lunar.day", DynamicValue::Integer(lunar_day_num))?;
        self.set_cache_field("lunar.ganzhi", DynamicValue::String(ganzhi_str))?;
        self.set_cache_field("lunar.zodiac", DynamicValue::String(zodiac_str))?;

        // 4. 更新节气字段
        let jieqi_str = {
            let solar_day = self
                .sxtwl_solar_day
                .as_ref()
                .ok_or(AppError::LunarCalculationError)?;

            let solar_term_day = solar_day.get_term_day();
            let jieqi_name = if solar_term_day.get_day_index() == 0 {
                solar_term_day.get_name()
            } else {
                "".to_string()
            };

            String::from_str(&jieqi_name).map_err(|_| AppError::LunarCalculationError)?
        };
        self.set_cache_field("lunar.jieqi", DynamicValue::String(jieqi_str))?;

        // 5. 更新节日字段
        let festival_str = {
            let lunar_day = self
                .sxtwl_lunar_day
                .as_ref()
                .ok_or(AppError::LunarCalculationError)?;

            let festival_name = if let Some(festival) = lunar_day.get_festival() {
                festival.get_name()
            } else {
                "".to_string()
            };

            String::from_str(&festival_name).map_err(|_| AppError::LunarCalculationError)?
        };
        self.set_cache_field("lunar.festival", DynamicValue::String(festival_str))?;

        // 6. 更新缓存的日期状态
        self.current_date = Some((year, month, day));
        log::debug!("Date fields updated: {}-{:02}-{:02}", year, month, day);

        Ok(())
    }
}

#[async_trait(?Send)]
impl DataSource for TimeDataSource {
    /// 获取数据源名称
    fn name(&self) -> &'static str {
        "datetime"
    }

    /// 获取字段值
    fn get_field_value(&self, name: &str) -> Result<DynamicValue> {
        self.cache
            .get_field(name)
            .cloned()
            .ok_or(AppError::FieldNotFound)
    }

    /// 刷新数据源
    async fn refresh(&mut self, _system_api: &'static GlobalMutex<DefaultSystemApi>) -> Result<()> {
        // 1. 获取实际时间戳并转换为东8区时间
        let time_driver_guard = self.time_source.lock().await;
        let timestamp = time_driver_guard.get_time().map_err(|e| {
            log::error!("Time driver error: {:?}", e);
            AppError::TimeDriverError
        })?;
        drop(time_driver_guard); // 尽早释放时间驱动锁

        let zoned = timestamp.to_zoned(TimeZone::fixed(Offset::constant(8)));
        let datetime: DateTime = zoned.into();

        // 2. 提取时间/日期组件（处理类型转换）
        let year = datetime.year() as isize;
        let month = datetime.month() as usize;
        let day = datetime.day() as usize;
        let hour = datetime.hour() as i32;
        let minute = datetime.minute() as i32;
        let weekday = datetime.weekday() as i32; // jiff: 0=周一, 6=周日

        // 3. 每次刷新：仅更新时间字段（核心优化：无论是否跨日都更新）
        self.set_cache_field("time.hour", DynamicValue::Integer(hour))?;
        self.set_cache_field("time.minute", DynamicValue::Integer(minute))?;
        self.set_cache_field("time.am_pm", DynamicValue::Boolean(hour < 12))?;

        // 4. 判断是否需要更新日期字段（首次刷新 或 跨日）
        let need_update_date = match self.current_date {
            None => true, // 首次刷新：需要初始化日期
            Some((cached_year, cached_month, cached_day)) => {
                // 跨日判断：年/月/日任一不一致则需要更新
                cached_year != year || cached_month != month || cached_day != day
            }
        };

        if need_update_date {
            // 仅跨日/首次时更新日期字段
            self.update_date_fields(year, month, day, weekday)?;
            log::info!(
                "Time data refreshed (date updated): {}-{:02}-{:02} {:02}:{:02}",
                year,
                month,
                day,
                hour,
                minute
            );
        } else {
            // 非跨日：仅更新时间字段，日志简化
            log::debug!("Time data refreshed (time only): {:02}:{:02}", hour, minute);
        }

        // 5. 更新缓存状态
        self.cache.mark_valid(Instant::now());

        Ok(())
    }

    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> Duration {
        Duration::from_secs(60) // 每分钟刷新一次
    }
}
