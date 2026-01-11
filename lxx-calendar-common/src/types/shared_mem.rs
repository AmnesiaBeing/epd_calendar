use crate::types::{
    DateTime, DisplayConfig, ForecastDay, Holiday, LogConfig, LunarDate, NetworkConfig,
    PowerConfig, SolarTerm, SystemMode, TimeConfig, WakeupReason,
};

#[repr(C)]
pub struct SharedMemory {
    pub system_state: SystemState,
    pub time_data: TimeData,
    pub weather_data: WeatherData,
    pub config_data: ConfigData,
    pub reserved: [u8; 64],
}

#[repr(C)]
pub struct SystemState {
    pub current_mode: SystemMode,
    pub last_wakeup_reason: WakeupReason,
    pub battery_level: u8,
    pub charging: bool,
    pub low_power_mode: bool,
}

#[repr(C)]
pub struct TimeData {
    pub current_time: DateTime,
    pub lunar_date: LunarDate,
    pub solar_term: Option<SolarTerm>,
    pub holiday: Option<Holiday>,
}

#[repr(C)]
pub struct WeatherData {
    pub forecast: heapless::Vec<ForecastDay, 3>,
    pub last_update: i64,
}

#[repr(C)]
pub struct ConfigData {
    pub time_config: TimeConfig,
    pub network_config: NetworkConfig,
    pub display_config: DisplayConfig,
    pub power_config: PowerConfig,
    pub log_config: LogConfig,
}
