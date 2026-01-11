use crate::types::{DateTime, LunarDate, WeatherInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayData {
    pub time: DateTime,
    pub lunar_date: LunarDate,
    pub weather: Option<WeatherInfo>,
    pub quote: Option<heapless::String<128>>,
    pub layout: DisplayLayout,
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
