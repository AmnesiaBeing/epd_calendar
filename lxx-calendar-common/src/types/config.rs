use crate::types::AlarmInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemConfig {
    pub version: u32,
    pub time_config: TimeConfig,
    pub network_config: NetworkConfig,
    pub display_config: DisplayConfig,
    pub power_config: PowerConfig,
    pub log_config: LogConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeConfig {
    pub timezone_offset: i32,
    pub alarms: heapless::Vec<AlarmInfo, 10>,
    pub hour_chime_enabled: bool,
    pub auto_sleep_start: Option<(u8, u8)>,
    pub auto_sleep_end: Option<(u8, u8)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub wifi_ssid: heapless::String<32>,
    pub wifi_password: EncryptedString,
    pub location_id: heapless::String<16>,
    pub sync_interval_minutes: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedString {
    pub data: heapless::Vec<u8, 64>,
    pub iv: heapless::Vec<u8, 16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub low_power_refresh_enabled: bool,
    pub refresh_interval_seconds: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerConfig {
    pub low_battery_threshold: u8,
    pub critical_battery_threshold: u8,
    pub low_power_mode_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogConfig {
    pub log_mode: LogMode,
    pub log_level: LogLevel,
    pub log_to_flash: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogMode {
    Log,
    Defmt,
    NoLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigChange {
    TimeConfig,
    NetworkConfig,
    DisplayConfig,
    PowerConfig,
    LogConfig,
}
