//! 编译期配置常量
//!
//! 这些值在编译时从 .env 文件读取，缺失会导致编译错误。
//!
//! # 环境变量
//!
//! - `QWEATHER_API_HOST`: 和风天气 API 域名
//! - `QWEATHER_LOCATION`: 默认城市 Location ID
//! - `QWEATHER_KEY_ID`: JWT 凭据 ID (kid)
//! - `QWEATHER_PROJECT_ID`: JWT 项目 ID (sub)
//! - `QWEATHER_PRIVATE_KEY`: Ed25519 私钥 (Base64 编码)

use crate::weather::QweatherJwtSigner;

/// 和风天气 API 域名
pub const QWEATHER_API_HOST: &str = env!("QWEATHER_API_HOST");

/// 默认城市 Location ID
pub const QWEATHER_LOCATION_DEFAULT: &str = env!("QWEATHER_LOCATION");

/// JWT 凭据 ID (kid)
pub const QWEATHER_KEY_ID: &str = env!("QWEATHER_KEY_ID");

/// JWT 项目 ID (sub)
pub const QWEATHER_PROJECT_ID: &str = env!("QWEATHER_PROJECT_ID");

/// Ed25519 私钥 (Base64 编码)
pub const QWEATHER_PRIVATE_KEY: &str = env!("QWEATHER_PRIVATE_KEY");

/// 创建和风天气 JWT 签名器
///
/// # Panics
///
/// 如果凭据格式无效，会在运行时 panic
pub fn create_qweather_jwt_signer() -> QweatherJwtSigner {
    QweatherJwtSigner::new(QWEATHER_KEY_ID, QWEATHER_PROJECT_ID, QWEATHER_PRIVATE_KEY)
        .expect("Invalid QWeather JWT credentials - check .env file")
}
