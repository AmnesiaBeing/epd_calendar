//! 编译期配置常量
//!
//! 这些值在编译时从 .env 文件读取，缺失会导致编译错误。
//!
//! # 环境变量
//!
//! - `OPENMETEO_LATITUDE`: 默认纬度
//! - `OPENMETEO_LONGITUDE`: 默认经度
//! - `OPENMETEO_LOCATION_NAME`: 默认位置名称
//!
//! ## 和风天气相关（已弃用，保留用于向后兼容）
//! - `QWEATHER_API_HOST`: 和风天气 API 域名
//! - `QWEATHER_LOCATION`: 默认城市 Location ID
//! - `QWEATHER_KEY_ID`: JWT 凭据 ID (kid)
//! - `QWEATHER_PROJECT_ID`: JWT 项目 ID (sub)
//! - `QWEATHER_PRIVATE_KEY`: Ed25519 私钥 (Base64 编码)

use crate::weather::QweatherJwtSigner;
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

// 和风天气配置（已弃用，保留用于向后兼容）
/// 和风天气 API 域名
pub fn qweather_api_host() -> &'static str {
    option_env!("QWEATHER_API_HOST").unwrap_or("devapi.qweather.com")
}

/// 默认城市 Location ID
pub fn qweather_location_default() -> &'static str {
    option_env!("QWEATHER_LOCATION").unwrap_or("101010100")
}

/// JWT 凭据 ID (kid)
pub fn qweather_key_id() -> &'static str {
    option_env!("QWEATHER_KEY_ID").unwrap_or("")
}

/// JWT 项目 ID (sub)
pub fn qweather_project_id() -> &'static str {
    option_env!("QWEATHER_PROJECT_ID").unwrap_or("")
}

/// Ed25519 私钥 (Base64 编码)
pub fn qweather_private_key() -> &'static str {
    option_env!("QWEATHER_PRIVATE_KEY").unwrap_or("")
}

/// 创建和风天气 JWT 签名器
///
/// # Panics
///
/// 如果凭据格式无效，会在运行时 panic
pub fn create_qweather_jwt_signer() -> QweatherJwtSigner {
    QweatherJwtSigner::new(
        qweather_key_id(),
        qweather_project_id(),
        qweather_private_key(),
    )
    .expect("Invalid QWeather JWT credentials - check .env file")
}
