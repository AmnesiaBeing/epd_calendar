// src/common/types.rs

use embassy_time::Instant;

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

#[derive(Debug, Clone)]
pub struct DisplayData {
    pub time: TimeData,
    pub weather: WeatherData,
    pub quote: String,
    pub status: StatusData,
    pub force_refresh: bool,
    pub last_display_update: Instant,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self {
            time: TimeData::default(),
            weather: WeatherData::default(),
            quote: String::new(),
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
    pub date_string: String,
    pub weekday: String,
    // pub holiday: Option<String>,
    // pub lunar: LunarData,
}

#[derive(Debug, Clone, Default)]
pub struct LunarData {
    pub year_name: String,
    pub zodiac: String,
    pub month: String,
    pub day: String,
    pub solar_term: Option<String>,
    pub suitable: Vec<String>,
    pub avoid: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WeatherData {
    // pub icon: WeatherIcon,
    pub temp_current: i8,
    pub temp_high: i8,
    pub temp_low: i8,
    pub humidity: u8,
    // pub wind_direction: WindDirection,
    pub wind_speed: u8,
    // pub air_quality: AirQuality,
}

#[derive(Debug, Clone, Default)]
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
