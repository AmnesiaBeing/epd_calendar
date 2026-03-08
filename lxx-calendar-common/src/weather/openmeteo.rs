use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenMeteoResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub generationtime_ms: f64,
    pub utc_offset_seconds: i32,
    pub timezone: String<32>,
    pub timezone_abbreviation: String<16>,
    pub elevation: f64,
    pub current_units: CurrentUnits,
    pub current: CurrentData,
    pub daily_units: DailyUnits,
    pub daily: DailyData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUnits {
    pub time: String<16>,
    pub interval: String<16>,
    pub temperature_2m: String<8>,
    pub relative_humidity_2m: String<8>,
    pub apparent_temperature: String<8>,
    pub weather_code: String<16>,
    pub wind_speed_10m: String<8>,
    pub wind_direction_10m: String<8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentData {
    pub time: String<32>,
    pub interval: i32,
    pub temperature_2m: f32,
    pub relative_humidity_2m: f32,
    pub apparent_temperature: f32,
    pub weather_code: u8,
    pub wind_speed_10m: f32,
    pub wind_direction_10m: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUnits {
    pub time: String<16>,
    pub weather_code: String<16>,
    pub temperature_2m_max: String<8>,
    pub temperature_2m_min: String<8>,
    pub precipitation_sum: String<8>,
    pub precipitation_probability_max: String<8>,
    pub wind_speed_10m_max: String<8>,
    pub sunrise: String<16>,
    pub sunset: String<16>,
    pub uv_index_max: String<16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyData {
    pub time: Vec<String<16>, 16>,
    pub weather_code: Vec<u8, 16>,
    pub temperature_2m_max: Vec<f32, 16>,
    pub temperature_2m_min: Vec<f32, 16>,
    pub precipitation_sum: Vec<f32, 16>,
    pub precipitation_probability_max: Vec<u8, 16>,
    pub wind_speed_10m_max: Vec<f32, 16>,
    pub sunrise: Vec<String<8>, 16>,
    pub sunset: Vec<String<8>, 16>,
    pub uv_index_max: Vec<f32, 16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyData {
    pub time: Vec<String<32>, 736>,
    pub temperature_2m: Vec<f32, 736>,
    pub weather_code: Vec<u8, 736>,
    pub precipitation_probability: Vec<u8, 736>,
}
