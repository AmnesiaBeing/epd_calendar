//! 编译期配置常量
//!
//! 这些值在编译时从 .env 文件读取，缺失会导致编译错误。
//!
//! # 环境变量
//!
//! - `OPENMETEO_LATITUDE`: 默认纬度
//! - `OPENMETEO_LONGITUDE`: 默认经度
//! - `OPENMETEO_LOCATION_NAME`: 默认位置名称

/// Open-Meteo 默认纬度
pub fn openmeteo_latitude() -> f64 {
    match option_env!("OPENMETEO_LATITUDE") {
        Some(lat_str) => lat_str.parse().unwrap_or(23.1291), // 默认广州
        None => 23.1291,
    }
}

/// Open-Meteo 默认经度
pub fn openmeteo_longitude() -> f64 {
    match option_env!("OPENMETEO_LONGITUDE") {
        Some(lon_str) => lon_str.parse().unwrap_or(113.2644), // 默认广州
        None => 113.2644,
    }
}

/// Open-Meteo 默认位置名称
pub fn openmeteo_location_name() -> &'static str {
    option_env!("OPENMETEO_LOCATION_NAME").unwrap_or("广州")
}
