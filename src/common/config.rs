// src/common/config.rs
//! 系统配置定义模块
//! 
//! 本模块提供：
//! - 系统配置结构体定义
//! - WiFi加密类型枚举
//! - 配置相关的常量和默认值
use heapless::String;
use serde::{Deserialize, Serialize};

/// 配置存储的魔法数字，用于验证配置的有效性
pub const CONFIG_MAGIC: u32 = 0x434F4E46; // "CONF" 的 ASCII

/// 配置数据最大大小（小于一个扇区）
pub const MAX_CONFIG_SIZE: usize = 512;

/// WiFi 加密类型
/// 
/// # 变体说明
/// - `None`: 无加密
/// - `WEP`: WEP加密
/// - `WPA`: WPA加密
/// - `WPA2`: WPA2加密
/// - `WPA3`: WPA3加密
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WifiEncryption {
    None = 0,
    WEP = 1,
    WPA = 2,
    WPA2 = 3,
    WPA3 = 4,
}

impl Default for WifiEncryption {
    /// 默认WiFi加密类型（WPA2）
    fn default() -> Self {
        Self::WPA2
    }
}

/// 系统配置结构体
/// 
/// 使用固定长度的字符串以避免动态内存分配
/// 
/// # 字段说明
/// - `wifi_ssid`: WiFi SSID（最多32字符）
/// - `wifi_password`: WiFi密码（最多64字符）
/// - `wifi_encryption`: WiFi加密类型
/// - `time_format_24h`: 是否使用24小时制
/// - `temperature_celsius`: 是否使用摄氏度
/// - `weather_api_key`: 天气API密钥（最多64字符）
/// - `weather_location`: 天气位置（最多32字符）
/// - `auto_refresh_interval`: 自动刷新间隔（秒）
/// - `partial_refresh_limit`: 局部刷新限制（次）
/// - `config_version`: 配置版本，用于配置迁移
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    // WiFi 配置
    pub wifi_ssid: String<32>,
    pub wifi_password: String<64>,
    pub wifi_encryption: WifiEncryption,

    // 显示配置
    pub time_format_24h: bool,
    pub temperature_celsius: bool,

    // 天气配置
    pub weather_api_key: String<64>,
    pub weather_location: String<32>,

    // 系统配置
    pub auto_refresh_interval: u32,
    pub partial_refresh_limit: u32,

    // 版本标记，用于配置迁移
    #[serde(default = "default_config_version")]
    pub config_version: u32,
}

impl Default for SystemConfig {
    /// 创建默认系统配置
    /// 
    /// # 返回值
    /// 返回默认配置实例
    fn default() -> Self {
        Self {
            wifi_ssid: String::new(),
            wifi_password: String::new(),
            wifi_encryption: WifiEncryption::default(),
            time_format_24h: true,
            temperature_celsius: true,
            weather_api_key: String::new(),
            weather_location: String::new(),
            auto_refresh_interval: 60, // 默认 60 秒
            partial_refresh_limit: 10, // 默认 10 次
            config_version: default_config_version(),
        }
    }
}

/// 获取默认配置版本号
/// 
/// # 返回值
/// 返回当前配置版本号
pub fn default_config_version() -> u32 {
    1 // 当前配置版本
}