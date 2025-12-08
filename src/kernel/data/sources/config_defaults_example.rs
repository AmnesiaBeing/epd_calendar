// src/core/data/sources/config_defaults_example.rs
//! 配置默认值注册示例
//! 展示各数据源如何注册自身的默认配置值

use crate::common::error::Result;
use crate::common::types::DynamicValue;
use crate::core::data::sources::config::ConfigDataSource;

/// Time数据源默认配置注册示例
pub fn register_time_defaults(config_data_source: &ConfigDataSource) -> Result<()> {
    // 注册时间相关默认配置
    config_data_source.register_default(
        "time.hour_format",
        DynamicValue::Uint32(24),
        "时间格式：12或24小时制"
    )?;
    
    config_data_source.register_default(
        "time.show_seconds",
        DynamicValue::Bool(true),
        "是否显示秒"
    )?;
    
    config_data_source.register_default(
        "time.refresh_interval",
        DynamicValue::Uint32(60),
        "时间刷新间隔（秒）"
    )?;
    
    Ok(())
}

/// Weather数据源默认配置注册示例
pub fn register_weather_defaults(config_data_source: &ConfigDataSource) -> Result<()> {
    // 注册天气相关默认配置
    config_data_source.register_default(
        "weather.api_key",
        DynamicValue::String(""),
        "天气API密钥"
    )?;
    
    config_data_source.register_default(
        "weather.location_id",
        DynamicValue::String("101010100"),
        "天气位置ID"
    )?;
    
    config_data_source.register_default(
        "weather.refresh_interval",
        DynamicValue::Uint32(1800),
        "天气刷新间隔（秒）"
    )?;
    
    config_data_source.register_default(
        "weather.display.temperature",
        DynamicValue::Bool(true),
        "是否显示温度"
    )?;
    
    config_data_source.register_default(
        "weather.display.humidity",
        DynamicValue::Bool(true),
        "是否显示湿度"
    )?;
    
    config_data_source.register_default(
        "weather.display.pressure",
        DynamicValue::Bool(false),
        "是否显示气压"
    )?;
    
    config_data_source.register_default(
        "weather.display.wind_speed",
        DynamicValue::Bool(false),
        "是否显示风速"
    )?;
    
    Ok(())
}
