// src/service/weather_service.rs
use crate::common::error::{AppError, Result};
use crate::driver::network::NetworkDriver;
use core::str::FromStr;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use log::{debug, info, warn};
use serde::Deserialize;

// 对外暴露的天气数据结构
#[derive(Debug, Clone, Default)]
pub struct WeatherData {
    /// 地区ID
    pub location_id: heapless::String<20>,
    /// 数据更新时间
    pub update_time: heapless::String<20>,
    /// 3天预报数据
    pub daily_forecast: heapless::Vec<DailyWeather, 3>,
}

pub struct WeatherService {
    network: &'static Mutex<ThreadModeRawMutex, NetworkDriver>,
    temperature_celsius: bool,
    location_id: heapless::String<20>,
    last_weather_data: Option<WeatherData>,
}

const API_HOST: &str = "devapi.qweather.com";
const API_PATH: &str = "/v7/weather/3d";
const API_KAY: &str = "";

// 天气图标枚举（适配和风天气icon代码）
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherIcon {
    Sunny,        // 100
    Cloudy,       // 101
    FewClouds,    // 102
    Overcast,     // 103
    Fog,          // 104
    LightRain,    // 300
    ModerateRain, // 301
    HeavyRain,    // 302
    Snow,         // 400
    Unknown,      // 其他未定义值
}

// 风向枚举
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum WindDirection {
    North,     // 北风
    Northeast, // 东北风
    East,      // 东风
    Southeast, // 东南风
    South,     // 南风
    Southwest, // 西南风
    West,      // 西风
    Northwest, // 西北风
    Unknown,   // 未知风向
}

// 单天天气预报
#[derive(Debug, Clone, Deserialize)]
pub struct DailyWeather {
    /// 预报日期（yyyy-MM-dd）
    #[serde(rename = "fxDate")]
    pub date: heapless::String<10>,
    /// 最高温度（℃）
    #[serde(rename = "tempMax")]
    pub temp_max: i8,
    /// 最低温度（℃）
    #[serde(rename = "tempMin")]
    pub temp_min: i8,
    /// 白天天气图标
    #[serde(rename = "iconDay")]
    pub icon_day: WeatherIcon,
    /// 白天天气描述
    #[serde(rename = "textDay")]
    pub text_day: heapless::String<20>,
    /// 夜间天气图标
    #[serde(rename = "iconNight")]
    pub icon_night: WeatherIcon,
    /// 夜间天气描述
    #[serde(rename = "textNight")]
    pub text_night: heapless::String<20>,
    /// 白天风向
    #[serde(rename = "windDirDay")]
    pub wind_direction: WindDirection,
    /// 白天风速（km/h）
    #[serde(rename = "windSpeedDay")]
    pub wind_speed: u8,
    /// 相对湿度（%）
    pub humidity: u8,
    /// 降水量（mm）
    pub precip: f32,
    /// 紫外线指数
    #[serde(rename = "uvIndex")]
    pub uv_index: u8,
}

// 和风天气API响应根结构
#[derive(Debug, Deserialize)]
struct QWeatherResponse {
    /// 状态码（200=成功）
    pub code: heapless::String<3>,
    /// API更新时间（yyyy-MM-ddTHH:mm+08:00）
    #[serde(rename = "updateTime")]
    pub update_time: heapless::String<20>,
    /// 3天预报数据
    pub daily: heapless::Vec<DailyWeather, 3>,
}

impl WeatherService {
    pub fn new(
        network: &'static Mutex<ThreadModeRawMutex, NetworkDriver>,
        temperature_celsius: bool,
    ) -> Self {
        Self {
            network,
            temperature_celsius,
            location_id: heapless::String::new(),
            last_weather_data: None,
        }
    }

    pub async fn get_weather(&mut self) -> Result<WeatherData> {
        // 首先检查网络连接
        if !self.network.lock().await.is_connected().await {
            warn!("Network not available, using cached or default weather data");
            return self.get_fallback_weather().await;
        }

        // 尝试从API获取天气数据
        match self.fetch_weather_from_api().await {
            Ok(weather) => {
                debug!("Successfully fetched weather data from API");
                self.last_weather_data = Some(weather.clone());
                Ok(weather)
            }
            Err(e) => {
                warn!("Failed to fetch weather from API: {}, using fallback", e);
                self.get_fallback_weather().await
            }
        }
    }

    pub fn set_temperature_celsius(&mut self, enabled: bool) {
        self.temperature_celsius = enabled;
    }

    pub fn has_valid_data(&self) -> bool {
        self.last_weather_data.is_some()
    }

    async fn fetch_weather_from_api(&self) -> Result<WeatherData> {
        // 检查是否有必要的配置
        if self.location_id.is_empty() {
            return Err(AppError::ConfigError("Weather API key or location not set"));
        }

        // 构建API请求URL
        let path = format!(
            "{}{}?key={}&location={}",
            API_HOST, API_PATH, API_KAY, self.location_id
        );

        // 发送HTTP请求
        debug!(
            "Fetching weather from API for location: {}",
            self.location_id
        );

        let mut buffer: [u8; 4096] = [0; 4096];

        let response = self
            .network
            .lock()
            .await
            .https_get("devapi.qweather.com", &path, &mut buffer)
            .await
            .map_err(|e| {
                warn!("HTTP request failed: {:?}", e);
                AppError::NetworkError
            })?;

        // 解析响应
        self.parse_weather_response(&response)
    }

    async fn get_fallback_weather(&self) -> Result<WeatherData> {
        // 返回缓存的天气数据或默认数据
        if let Some(ref weather) = self.last_weather_data {
            debug!("Using cached weather data");
            Ok(weather.clone())
        } else {
            debug!("Using default weather data");
            Ok(self.create_default_weather_data())
        }
    }

    /// 解析和风天气API的JSON响应
    fn parse_weather_response(&self, response: &[u8]) -> Result<WeatherData> {
        info!("Parsing weather response ({} bytes)", response.len());

        // 反序列化JSON
        let qweather_resp: QWeatherResponse = serde_json::from_slice(response).map_err(|e| {
            warn!("JSON parse error: {:?}", e);
            AppError::WeatherApiError
        })?;

        // 2. 校验API状态码
        if qweather_resp.code != "200" {
            log::warn!("API returned error code: {}", qweather_resp.code);
            return Err(AppError::WeatherApiError);
        }

        // 3. 转换为对外的WeatherData结构
        let mut weather_data = WeatherData {
            location_id: self.location_id.clone(),
            update_time: qweather_resp.update_time,
            daily_forecast: heapless::Vec::new(),
        };

        // 4. 填充3天预报数据
        for daily in qweather_resp.daily {
            weather_data
                .daily_forecast
                .push(daily)
                .map_err(|_| AppError::WeatherApiError)?;
        }

        // 5. 校验至少有1天数据
        if weather_data.daily_forecast.is_empty() {
            log::warn!("No daily weather data found in API response");
            return Err(AppError::WeatherApiError);
        }

        Ok(weather_data)
    }

    /// 创建默认天气数据（无网络时使用）
    fn create_default_weather_data(&self) -> WeatherData {
        let mut weather_data = WeatherData::default();
        weather_data.location_id = self.location_id.clone();
        weather_data.update_time = heapless::String::from_str("2024-01-01T00:00+08:00").unwrap();

        // 默认3天数据
        let default_days = [
            DailyWeather {
                date: heapless::String::from_str("2024-01-01").unwrap(),
                temp_max: 20,
                temp_min: 10,
                icon_day: WeatherIcon::Sunny,
                text_day: heapless::String::from_str("晴").unwrap(),
                icon_night: WeatherIcon::Cloudy,
                text_night: heapless::String::from_str("多云").unwrap(),
                wind_direction: WindDirection::East,
                wind_speed: 2,
                humidity: 60,
                precip: 0.0,
                uv_index: 2,
            },
            DailyWeather {
                date: heapless::String::from_str("2024-01-02").unwrap(),
                temp_max: 19,
                temp_min: 9,
                icon_day: WeatherIcon::Cloudy,
                text_day: heapless::String::from_str("多云").unwrap(),
                icon_night: WeatherIcon::Cloudy,
                text_night: heapless::String::from_str("多云").unwrap(),
                wind_direction: WindDirection::Southeast,
                wind_speed: 3,
                humidity: 65,
                precip: 0.0,
                uv_index: 2,
            },
            DailyWeather {
                date: heapless::String::from_str("2024-01-03").unwrap(),
                temp_max: 18,
                temp_min: 8,
                icon_day: WeatherIcon::FewClouds,
                text_day: heapless::String::from_str("少云").unwrap(),
                icon_night: WeatherIcon::Sunny,
                text_night: heapless::String::from_str("晴").unwrap(),
                wind_direction: WindDirection::South,
                wind_speed: 2,
                humidity: 70,
                precip: 0.0,
                uv_index: 1,
            },
        ];

        for day in default_days {
            let _ = weather_data.daily_forecast.push(day);
        }

        weather_data
    }

    // 温度单位转换
    pub fn convert_temperature(&self, temp: i8) -> i8 {
        if self.temperature_celsius {
            temp
        } else {
            // 摄氏度转华氏度
            (temp * 9 / 5) + 32
        }
    }
}
