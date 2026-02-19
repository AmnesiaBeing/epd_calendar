#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentWeather {
    pub temp: i16,
    pub feels_like: i16,
    pub humidity: u8,
    pub condition: WeatherCondition,
    pub wind_speed: u8,
    pub wind_direction: u16,
    pub visibility: u16,
    pub pressure: u16,
    pub update_time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherInfo {
    pub location: heapless::String<32>,
    pub current: CurrentWeather,
    pub forecast: heapless::Vec<ForecastDay, 3>,
    pub last_update: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForecastDay {
    pub date: i64,
    pub high_temp: i16,
    pub low_temp: i16,
    pub condition: WeatherCondition,
    pub humidity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherCondition {
    Sunny,
    Cloudy,
    Overcast,
    LightRain,
    ModerateRain,
    HeavyRain,
    Thunderstorm,
    Snow,
    Fog,
    Haze,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncResult {
    pub time_synced: bool,
    pub weather_synced: bool,
    pub quote_updated: bool,
    pub sync_duration: embassy_time::Duration,
}
