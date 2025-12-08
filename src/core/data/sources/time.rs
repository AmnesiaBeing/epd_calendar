// src/core/data/sources/time.rs
//! 时间数据源模块
//! 提供时间和日期相关数据的数据源实现

use crate::common::error::{AppError, Result};
use crate::common::GlobalMutex;
use crate::core::data::source::{DataSource, DataSourceCache};
use crate::core::data::types::{DataSourceId, DynamicValue, FieldMeta, FieldType};
use crate::driver::time_source::{DefaultTimeSource, TimeSource};
use heapless::{String, Vec};
use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};

/// 时间数据源结构体
pub struct TimeDataSource {
    /// 时间源驱动实例（全局互斥锁保护）
    time_source: &'static GlobalMutex<DefaultTimeSource>,
    /// 数据源缓存
    cache: DataSourceCache,
    /// 字段元数据列表
    fields: Vec<FieldMeta, 15>,
    /// 上次农历刷新的日期
    last_lunar_refresh_day: u8,
}

impl TimeDataSource {
    /// 创建新的时间数据源实例
    pub fn new(time_source: &'static GlobalMutex<DefaultTimeSource>) -> Self {
        // 初始化字段元数据
        let mut fields = Vec::new();
        
        // 时间相关字段
        fields.push(FieldMeta {
            name: String::from("time.hour"),
            field_type: FieldType::Integer,
            format: String::from("%02d"),
            nullable: false,
            description: String::from("当前小时"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("time.minute"),
            field_type: FieldType::Integer,
            format: String::from("%02d"),
            nullable: false,
            description: String::from("当前分钟"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("time.am_pm"),
            field_type: FieldType::Boolean,
            format: String::from(""),
            nullable: true,
            description: String::from("是否为下午"),
        }).unwrap();
        
        // 日期相关字段
        fields.push(FieldMeta {
            name: String::from("date.year"),
            field_type: FieldType::Integer,
            format: String::from("%d"),
            nullable: false,
            description: String::from("当前年份"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("date.month"),
            field_type: FieldType::Integer,
            format: String::from("%02d"),
            nullable: false,
            description: String::from("当前月份"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("date.day"),
            field_type: FieldType::Integer,
            format: String::from("%02d"),
            nullable: false,
            description: String::from("当前日期"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("date.week"),
            field_type: FieldType::Integer,
            format: String::from(""),
            nullable: false,
            description: String::from("当前星期"),
        }).unwrap();
        
        // 农历相关字段
        fields.push(FieldMeta {
            name: String::from("lunar.year"),
            field_type: FieldType::LunarDate,
            format: String::from("%d"),
            nullable: false,
            description: String::from("农历年份"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("lunar.month"),
            field_type: FieldType::LunarDate,
            format: String::from("%d"),
            nullable: false,
            description: String::from("农历月份"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("lunar.day"),
            field_type: FieldType::LunarDate,
            format: String::from("%d"),
            nullable: false,
            description: String::from("农历日期"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("lunar.ganzhi"),
            field_type: FieldType::String,
            format: String::from(""),
            nullable: false,
            description: String::from("农历干支"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("lunar.zodiac"),
            field_type: FieldType::String,
            format: String::from(""),
            nullable: false,
            description: String::from("农历生肖"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("lunar.jieqi"),
            field_type: FieldType::String,
            format: String::from(""),
            nullable: true,
            description: String::from("当前节气"),
        }).unwrap();
        
        fields.push(FieldMeta {
            name: String::from("lunar.festival"),
            field_type: FieldType::String,
            format: String::from(""),
            nullable: true,
            description: String::from("农历节日"),
        }).unwrap();
        
        Self {
            time_source,
            cache: DataSourceCache::default(),
            fields,
            last_lunar_refresh_day: 0,
        }
    }
    
    /// 获取当前时间数据
    async fn get_current_time(&self) -> Result<(u8, u8, Option<bool>)> {
        let datetime = self
            .time_source
            .lock()
            .await
            .get_time()
            .map_err(|_| AppError::TimeError)?;

        let zoned = datetime.to_zoned(TimeZone::fixed(Offset::constant(8)));
        let datetime: DateTime = zoned.into();

        Ok((datetime.hour() as u8, datetime.minute() as u8, None))
    }
    
    /// 刷新农历数据
    fn refresh_lunar_data(&mut self, day: u8) -> Result<()> {
        // 这里应该添加农历数据的刷新逻辑
        // 目前使用默认值
        
        // 更新缓存
        self.cache.set_field(String::from("lunar.year"), DynamicValue::Integer(2025))?;
        self.cache.set_field(String::from("lunar.month"), DynamicValue::Integer(1))?;
        self.cache.set_field(String::from("lunar.day"), DynamicValue::Integer(1))?;
        self.cache.set_field(String::from("lunar.ganzhi"), DynamicValue::String(String::from("乙巳")))?;
        self.cache.set_field(String::from("lunar.zodiac"), DynamicValue::String(String::from("蛇")))?;
        self.cache.set_field(String::from("lunar.jieqi"), DynamicValue::String(String::from("立春")))?;
        self.cache.set_field(String::from("lunar.festival"), DynamicValue::None)?;
        
        // 更新上次刷新日期
        self.last_lunar_refresh_day = day;
        
        Ok(())
    }
}

impl DataSource for TimeDataSource {
    /// 获取数据源ID
    fn id(&self) -> DataSourceId {
        DataSourceId::Time
    }
    
    /// 获取数据源名称
    fn name(&self) -> &'static str {
        "System Time & Lunar Data Source"
    }
    
    /// 获取字段元数据列表
    fn fields(&self) -> &[FieldMeta] {
        &self.fields
    }
    
    /// 获取字段值
    fn get_field_value(&self, name: &str) -> Result<DynamicValue> {
        self.cache
            .get_field(name)
            .ok_or(AppError::FieldNotFound)
            .cloned()
    }
    
    /// 刷新数据源
    async fn refresh(&mut self, system_api: &dyn crate::core::system::api::SystemApi) -> Result<()> {
        // 从配置数据源获取时间格式
        let time_display_mode = match system_api.get_data_source_registry().get_data_source(crate::core::data::types::DataSourceId::Config) {
            Some(config_source) => {
                match config_source.get_field_value("config.time.hour_format") {
                    Ok(DynamicValue::Integer(mode)) => mode,
                    _ => {
                        log::warn!("Failed to get time display mode, using 24h format by default");
                        24
                    }
                }
            },
            None => {
                log::warn!("Config data source not found, using 24h format by default");
                24
            }
        };
        
        // 使用SystemApi获取时间戳
        let timestamp = system_api.get_utc_timestamp();
        
        // 根据时间戳计算时间字段
        let datetime = jiff::Timestamp::from_seconds(timestamp as i64)
            .map_err(|_| AppError::TimeError)?;
        
        let zoned = datetime.to_zoned(TimeZone::fixed(Offset::constant(8)));
        let datetime: DateTime = zoned.into();
        
        // 根据配置计算小时显示
        let (hour, am_pm) = if time_display_mode == 12 {
            let h = datetime.hour();
            if h == 0 {
                (12, Some(false)) // 12 AM
            } else if h < 12 {
                (h, Some(false)) // AM
            } else if h == 12 {
                (12, Some(true)) // 12 PM
            } else {
                (h - 12, Some(true)) // PM
            }
        } else {
            (datetime.hour(), None) // 24小时制
        };
        
        // 更新时间字段
        self.cache.set_field(String::from("time.hour"), DynamicValue::Integer(hour as i64))?;
        self.cache.set_field(String::from("time.minute"), DynamicValue::Integer(datetime.minute() as i64))?;
        self.cache.set_field(String::from("time.am_pm"), match am_pm {
            Some(is_pm) => DynamicValue::Boolean(is_pm),
            None => DynamicValue::None,
        })?;
        
        // 更新日期字段
        self.cache.set_field(String::from("date.year"), DynamicValue::Integer(datetime.year() as i64))?;
        self.cache.set_field(String::from("date.month"), DynamicValue::Integer(datetime.month() as i64))?;
        self.cache.set_field(String::from("date.day"), DynamicValue::Integer(datetime.day() as i64))?;
        self.cache.set_field(String::from("date.week"), DynamicValue::Integer(datetime.weekday().to_num() as i64))?;
        
        // 检查是否需要刷新农历数据（跨日才刷新）
        if datetime.day() as u8 != self.last_lunar_refresh_day {
            self.refresh_lunar_data(datetime.day() as u8)?;
        }
        
        // 更新缓存状态
        self.cache.valid = true;
        self.cache.last_updated = system_api.get_system_ticks() as u32;
        
        log::info!("Time data refreshed successfully");
        Ok(())
    }
    
    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> u32 {
        60 // 每分钟刷新一次
    }
    
    /// 检查数据是否有效
    fn is_data_valid(&self) -> bool {
        self.cache.valid
    }
    
    /// 获取缓存
    fn get_cache(&self) -> &DataSourceCache {
        &self.cache
    }
    
    /// 获取可变缓存
    fn get_cache_mut(&mut self) -> &mut DataSourceCache {
        &mut self.cache
    }
}