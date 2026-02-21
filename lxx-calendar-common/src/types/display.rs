use crate::types::{
    LunarDay, LunarFestival, SolarFestival, SolarTerm, SolarTime, WeatherInfo, Week,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayData {
    pub solar_time: SolarTime,
    pub weekday: Week,
    pub lunar_date: LunarDay,
    pub weather: Option<WeatherInfo>,
    pub quote: Option<heapless::String<128>>,
    pub layout: DisplayLayout,
    pub solar_term: Option<SolarTerm>,
    pub lunar_festival: Option<LunarFestival>,
    pub solar_festival: Option<SolarFestival>,
    pub low_battery: bool,
    pub charging: bool,
    pub voltage: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayLayout {
    Default,
    LargeTime,
    WeatherFocus,
    QuoteFocus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshMode {
    Full,
    Partial,
    Fast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshState {
    Idle,
    SendingData,
    Refreshing,
    Error(RefreshError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshError {
    Timeout,
    CommunicationError,
    PowerError,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Melody {
    HourChime,
    Alarm1,
    Alarm2,
    Alarm3,
    Custom,
}
