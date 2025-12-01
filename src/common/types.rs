// src/common/types.rs

#[cfg(feature = "embedded_esp")]
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use crate::{
    assets::generated_hitokoto_data::{HITOKOTOS, Hitokoto},
    service::weather_service::WeatherData,
};

// 按照不同平台使用不同的锁
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type GlobalMutex<T> = Mutex<ThreadModeRawMutex, T>;
#[cfg(feature = "embedded_esp")]
pub type GlobalMutex<T> = Mutex<NoopRawMutex, T>;

#[derive(Debug, Clone, Default)]
pub struct SystemConfig {
    // WiFi配置
    // pub wifi_ssid: heapless::String<32>,
    // pub wifi_password: heapless::String<64>,
    // pub wifi_encryption: WifiEncryption,

    // 显示配置
    pub time_format_24h: bool,
    pub temperature_celsius: bool,
    pub show_am_pm: bool,

    // 天气API配置
    // pub weather_api_key: heapless::String<64>,
    // pub weather_location: heapless::String<32>,

    // 其他配置
    pub auto_refresh_interval: u32, // 分钟
    pub partial_refresh_limit: u32,
}

#[derive(Clone)]
pub struct DisplayData<'a> {
    pub time: TimeData,
    pub weather: WeatherData,
    pub quote: &'a Hitokoto,
    pub status: StatusData,
    pub force_refresh: bool,
    pub last_display_update: Instant,
}

impl Default for DisplayData<'_> {
    fn default() -> Self {
        Self {
            time: TimeData::default(),
            weather: WeatherData::default(),
            quote: &HITOKOTOS[0],
            status: StatusData::default(),
            force_refresh: false,
            last_display_update: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TimeData {
    pub hour: u8,
    pub minute: u8,
    pub is_24_hour: bool,
    // pub date_string: String,
    // pub weekday: &'static str,
    // pub holiday: Option<String>,
    // pub lunar: LunarData,
}

// #[derive(Debug, Clone, Default)]
// pub struct LunarData {
//     pub year_name: String,
//     pub zodiac: String,
//     pub month: String,
//     pub day: String,
//     pub solar_term: Option<String>,
//     pub suitable: Vec<String>,
//     pub avoid: Vec<String>,
// }

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatusData {
    pub is_charging: bool,
    pub battery_level: BatteryLevel,
    pub is_online: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum BatteryLevel {
    #[default]
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
}
