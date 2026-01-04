//! 时间数据源模块
//! 提供时间和日期相关数据的数据源实现

use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::Duration;
use heapless::format;
use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};
use sxtwl_rs::lunar::LunarDay;
use sxtwl_rs::solar::SolarDay;
use sxtwl_rs::types::{Culture, Tyme};

use crate::common::error::{AppError, Result};
use crate::common::{GlobalMutex, GlobalRwLockWriteGuard};
use crate::kernel::data::types::{
    CacheStringValue, HeaplessString, KEY_LENGTH, alloc_string_to_heapless,
};
use crate::kernel::data::{DataSource, DynamicValue, types::CacheKeyValueMap};
use crate::kernel::driver::time_driver::{DefaultTimeDriver, TimeDriver};
use crate::kernel::system::api::DefaultSystemApi;

// --------------- 常量定义 ---------------
const CACHE_KEY_YEAR: &str = "year";
const CACHE_KEY_MONTH: &str = "month";
const CACHE_KEY_DAY: &str = "day";
const CACHE_KEY_WEEKDAY: &str = "weekday";
const CACHE_KEY_LUNAR_MONTH: &str = "lunar.month";
const CACHE_KEY_LUNAR_DAY: &str = "lunar.day";
const CACHE_KEY_LUNAR_GANZHI: &str = "lunar.ganzhi";
const CACHE_KEY_LUNAR_ZODIAC: &str = "lunar.zodiac";
const CACHE_KEY_LUNAR_JIEQI: &str = "lunar.jieqi";
const CACHE_KEY_LUNAR_FESTIVAL: &str = "lunar.festival";
const CACHE_KEY_HOUR: &str = "hour";
const CACHE_KEY_MINUTE: &str = "minute";
const CACHE_KEY_AM_PM: &str = "am_pm";
const CACHE_KEY_HOUR_TENS: &str = "hour_tens";
const CACHE_KEY_HOUR_ONES: &str = "hour_ones";
const CACHE_KEY_MINUTE_TENS: &str = "minute_tens";
const CACHE_KEY_MINUTE_ONES: &str = "minute_ones";

// --------------- 魔法数字常量 ---------------
/// 节气日期索引为0表示当天是节气日
const SOLAR_TERM_DAY_INDEX_ACTIVE: usize = 0;

/// 时间数据源结构体（删除本地缓存，适配全局缓存直写）
pub struct TimeDataSource {
    /// 时间源驱动实例（全局互斥锁保护）
    time_source: &'static GlobalMutex<DefaultTimeDriver>,
    /// sxtwl 公历日期结构体
    sxtwl_solar_day: Option<SolarDay>,
    /// sxtwl 农历日期结构体
    sxtwl_lunar_day: Option<LunarDay>,
    /// 上一次更新农历的距今的日期
    last_update_lunar: u8,
    /// 当前缓存的公历日期（年、月、日），用于判断是否跨日
    current_date: Option<(isize, usize, usize)>, // (year, month, day)
}

impl TimeDataSource {
    /// 创建新的时间数据源实例（删除本地缓存初始化）
    pub fn new(time_source: &'static GlobalMutex<DefaultTimeDriver>) -> Result<Self> {
        Ok(Self {
            time_source,
            sxtwl_solar_day: None,
            sxtwl_lunar_day: None,
            last_update_lunar: 0,
            current_date: None,
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
        self.last_update_lunar = 0;

        Ok(())
    }

    /// 推算下一日的公历&农历数据
    fn next_date(&mut self) -> Result<()> {
        let Some(mut solar) = self.sxtwl_solar_day.take() else {
            log::error!("Solar data not initialized");
            return Err(AppError::LunarCalculationError);
        };
        let Some(mut lunar) = self.sxtwl_lunar_day.take() else {
            log::error!("Lunar data not initialized");
            return Err(AppError::LunarCalculationError);
        };

        if self.last_update_lunar <= 15 {
            solar = solar.next(1);
            lunar = lunar.next(1);
            self.last_update_lunar += 1;
        } else {
            solar = solar.next(1);
            lunar = solar.get_lunar_day();
            self.last_update_lunar = 0;
        }

        self.sxtwl_solar_day = Some(solar);
        self.sxtwl_lunar_day = Some(lunar);
        Ok(())
    }

    /// 辅助函数：安全写入全局缓存（替代原 set_cache_field）
    /// 自动拼接数据源名称前缀（datetime.xxx）
    fn write_global_cache(
        &self,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
        field_name: &str,
        value: DynamicValue,
    ) -> Result<()> {
        // 拼接全局缓存key：数据源名称.字段名（如 "datetime.time.hour"）
        let full_key = format!(KEY_LENGTH; "{}.{}", self.name(), field_name).unwrap();

        cache_guard.insert(full_key, value);
        Ok(())
    }

    /// 更新日期（公历+农历）字段到全局缓存（仅跨日时调用）
    ///
    /// # 参数
    /// - `cache_guard`: 全局缓存的写锁守卫
    /// - `year`: 公历年（isize类型，适配不同平台的整数范围）
    /// - `month`: 公历月（1-12）
    /// - `day`: 公历日（1-31）
    /// - `weekday`: 星期几（0-6 或 1-7，根据业务定义）
    ///
    /// # 返回值
    /// 成功返回Ok(())，失败返回AppError（包含农历计算、缓存写入等错误）
    fn update_date_fields(
        &mut self,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
        year: isize,
        month: usize,
        day: usize,
        weekday: i32,
    ) -> Result<()> {
        // 1. 初始化/更新公历&农历核心数据
        if self.sxtwl_solar_day.is_none() {
            self.init_solar_lunar(year, month, day)?;
        } else {
            self.next_date()?;
        }

        // 2. 提前获取核心数据（避免重复as_ref()+错误处理）
        let solar_day = self
            .sxtwl_solar_day
            .as_ref()
            .ok_or(AppError::LunarCalculationError)?;
        let lunar_day = self
            .sxtwl_lunar_day
            .as_ref()
            .ok_or(AppError::LunarCalculationError)?;

        // 3. 写入公历日期字段到全局缓存
        self.write_date_to_cache(cache_guard, year, month, day, weekday)?;

        // 4. 提取并写入农历核心数据
        self.write_lunar_core_data_to_cache(cache_guard, lunar_day)?;

        // 5. 写入节气字段
        let jieqi_str = self.get_solar_term_name(solar_day);
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_LUNAR_JIEQI,
            DynamicValue::String(jieqi_str),
        )?;

        // 6. 写入节日字段
        let festival_str = self.get_lunar_festival_name(lunar_day);
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_LUNAR_FESTIVAL,
            DynamicValue::String(festival_str),
        )?;

        // 7. 更新缓存的日期状态（仅当所有写入操作成功后更新）
        self.current_date = Some((year, month, day));

        Ok(())
    }

    /// 写入公历日期到缓存
    fn write_date_to_cache(
        &self,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
        year: isize,
        month: usize,
        day: usize,
        weekday: i32,
    ) -> Result<()> {
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_YEAR,
            DynamicValue::Integer(year as i32),
        )?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_MONTH,
            DynamicValue::Integer(month as i32),
        )?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_DAY,
            DynamicValue::Integer(day as i32),
        )?;

        const WEEKDAY_NAMES: &[&str] = &[
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ];

        self.write_global_cache(
            cache_guard,
            CACHE_KEY_WEEKDAY,
            DynamicValue::String(alloc_string_to_heapless(WEEKDAY_NAMES[weekday as usize])?),
        )?;

        Ok(())
    }

    /// 提取并写入农历核心数据（干支、生肖、月/日）
    fn write_lunar_core_data_to_cache(
        &self,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
        lunar_day: &LunarDay, // 替换为实际的LunarDay类型
    ) -> Result<()> {
        // 提取农历月/日
        let lunar_month = lunar_day.get_month() as i32;
        let lunar_day_num = lunar_day.get_day() as i32;

        // 提取干支信息
        let ganzhi_str = self.get_heapless_string(
            lunar_day
                .get_lunar_month()
                .get_lunar_year()
                .get_sixty_cycle()
                .get_name()
                .as_str(),
        )?;

        // 提取生肖信息
        let zodiac_str = self.get_heapless_string(
            lunar_day
                .get_lunar_month()
                .get_lunar_year()
                .get_sixty_cycle()
                .get_earth_branch()
                .get_zodiac()
                .get_name()
                .as_str(),
        )?;

        // 写入缓存
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_LUNAR_MONTH,
            DynamicValue::Integer(lunar_month),
        )?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_LUNAR_DAY,
            DynamicValue::Integer(lunar_day_num),
        )?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_LUNAR_GANZHI,
            DynamicValue::String(ganzhi_str),
        )?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_LUNAR_ZODIAC,
            DynamicValue::String(zodiac_str),
        )?;

        Ok(())
    }

    /// 封装heapless字符串转换（统一错误处理）
    fn get_heapless_string(&self, s: &str) -> Result<CacheStringValue> {
        alloc_string_to_heapless(s).map_err(|e| {
            log::error!("Failed to allocate heapless string for '{}': {}", s, e);
            AppError::LunarCalculationError
        })
    }

    /// 获取节气名称（空则返回空字符串）
    fn get_solar_term_name(&self, solar_day: &SolarDay) -> CacheStringValue {
        // 替换为实际的SolarDay类型
        let solar_term_day = solar_day.get_term_day();
        if solar_term_day.get_day_index() == SOLAR_TERM_DAY_INDEX_ACTIVE {
            self.get_heapless_string(solar_term_day.get_name().as_str())
                .unwrap_or_default()
        } else {
            HeaplessString::new()
        }
    }

    /// 获取农历节日名称（空则返回空字符串）
    fn get_lunar_festival_name(&self, lunar_day: &LunarDay) -> CacheStringValue {
        lunar_day.get_festival().map_or(HeaplessString::new(), |f| {
            self.get_heapless_string(f.get_name().as_str())
                .unwrap_or_default()
        })
    }
}

#[async_trait(?Send)]
impl DataSource for TimeDataSource {
    /// 获取数据源名称（用于拼接全局缓存key）
    fn name(&self) -> &'static str {
        "datetime"
    }

    /// 获取刷新间隔（保持原有逻辑：每分钟刷新）
    fn refresh_interval(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// 核心变更：刷新数据并直接写入全局缓存（替代原 refresh 方法）
    async fn refresh_with_cache(
        &mut self,
        _system_api: &'static GlobalMutex<DefaultSystemApi>,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
    ) -> Result<()> {
        // 1. 获取实际时间戳并转换为东8区时间（保持原有逻辑）
        let time_driver_guard = self.time_source.lock().await;
        let timestamp = time_driver_guard.get_time().map_err(|e| {
            log::error!("Time driver error: {:?}", e);
            AppError::TimeDriverError
        })?;
        drop(time_driver_guard);

        let zoned = timestamp.to_zoned(TimeZone::fixed(Offset::constant(8)));
        let datetime: DateTime = zoned.into();

        // 2. 提取时间/日期组件（保持原有逻辑）
        let year = datetime.year() as isize;
        let month = datetime.month() as usize;
        let day = datetime.day() as usize;
        let hour = datetime.hour() as i32;
        let minute = datetime.minute() as i32;
        let weekday = datetime.weekday() as i32; // jiff: 0=周一, 6=周日

        // 3. 写入时间字段到全局缓存（每次刷新都更新）
        self.write_global_cache(cache_guard, CACHE_KEY_HOUR, DynamicValue::Integer(hour))?;
        self.write_global_cache(cache_guard, CACHE_KEY_MINUTE, DynamicValue::Integer(minute))?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_AM_PM,
            DynamicValue::Boolean(hour < 12),
        )?;

        // 4. 写入时间拆分项到全局缓存
        let hour_tens = (hour / 10) as i32;
        let hour_ones = (hour % 10) as i32;
        let minute_tens = (minute / 10) as i32;
        let minute_ones = (minute % 10) as i32;

        self.write_global_cache(
            cache_guard,
            CACHE_KEY_HOUR_TENS,
            DynamicValue::Integer(hour_tens),
        )?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_HOUR_ONES,
            DynamicValue::Integer(hour_ones),
        )?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_MINUTE_TENS,
            DynamicValue::Integer(minute_tens),
        )?;
        self.write_global_cache(
            cache_guard,
            CACHE_KEY_MINUTE_ONES,
            DynamicValue::Integer(minute_ones),
        )?;

        // 4. 判断是否需要更新日期字段（首次刷新 或 跨日）
        let need_update_date = match self.current_date {
            None => true,
            Some((cached_year, cached_month, cached_day)) => {
                cached_year != year || cached_month != month || cached_day != day
            }
        };

        if need_update_date {
            // 仅跨日/首次时更新日期字段到全局缓存
            self.update_date_fields(cache_guard, year, month, day, weekday)?;
            log::info!(
                "Time data refreshed (date updated): {}-{:02}-{:02} {:02}:{:02}",
                year,
                month,
                day,
                hour,
                minute
            );
        } else {
            // 非跨日：仅更新时间字段
            log::debug!("Time data refreshed (time only): {:02}:{:02}", hour, minute);
        }

        Ok(())
    }
}
