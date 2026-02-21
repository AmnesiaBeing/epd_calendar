use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherDailyResponse {
    pub code: HeaplessString<8>,
    #[serde(rename = "updateTime")]
    pub update_time: HeaplessString<32>,
    #[serde(rename = "fxLink")]
    pub fx_link: Option<HeaplessString<128>>,
    pub daily: Vec<DailyForecast>,
    pub refer: Option<Refer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyForecast {
    #[serde(rename = "fxDate")]
    pub fx_date: HeaplessString<16>,
    pub sunrise: Option<HeaplessString<8>>,
    pub sunset: Option<HeaplessString<8>>,
    pub moonrise: Option<HeaplessString<8>>,
    pub moonset: Option<HeaplessString<8>>,
    pub moon_phase: Option<HeaplessString<16>>,
    #[serde(rename = "moonPhaseIcon")]
    pub moon_phase_icon: Option<HeaplessString<8>>,
    #[serde(rename = "tempMax")]
    pub temp_max: HeaplessString<8>,
    #[serde(rename = "tempMin")]
    pub temp_min: HeaplessString<8>,
    #[serde(rename = "iconDay")]
    pub icon_day: HeaplessString<8>,
    #[serde(rename = "textDay")]
    pub text_day: HeaplessString<32>,
    #[serde(rename = "iconNight")]
    pub icon_night: Option<HeaplessString<8>>,
    #[serde(rename = "textNight")]
    pub text_night: Option<HeaplessString<32>>,
    #[serde(rename = "wind360Day")]
    pub wind_360_day: Option<HeaplessString<8>>,
    #[serde(rename = "windDirDay")]
    pub wind_dir_day: Option<HeaplessString<32>>,
    #[serde(rename = "windScaleDay")]
    pub wind_scale_day: Option<HeaplessString<8>>,
    #[serde(rename = "windSpeedDay")]
    pub wind_speed_day: Option<HeaplessString<8>>,
    #[serde(rename = "wind360Night")]
    pub wind_360_night: Option<HeaplessString<8>>,
    #[serde(rename = "windDirNight")]
    pub wind_dir_night: Option<HeaplessString<32>>,
    #[serde(rename = "windScaleNight")]
    pub wind_scale_night: Option<HeaplessString<8>>,
    #[serde(rename = "windSpeedNight")]
    pub wind_speed_night: Option<HeaplessString<8>>,
    pub humidity: Option<HeaplessString<8>>,
    pub precip: Option<HeaplessString<8>>,
    pub pressure: Option<HeaplessString<8>>,
    pub vis: Option<HeaplessString<8>>,
    pub cloud: Option<HeaplessString<8>>,
    #[serde(rename = "uvIndex")]
    pub uv_index: Option<HeaplessString<8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refer {
    pub sources: Option<Vec<HeaplessString<32>>>,
    pub license: Option<Vec<HeaplessString<32>>>,
}

pub type HeaplessString<const N: usize> = heapless::String<N>;
