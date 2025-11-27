// src/service/weather_service.rs
use crate::common::error::{AppError, Result};
use crate::common::types::{AirQuality, WeatherData, WeatherIcon, WindDirection};
use crate::driver::network::NetworkDriver;
use log::{debug, warn};

pub struct WeatherService<T: NetworkDriver> {
    network: T,
    temperature_celsius: bool,
    api_key: heapless::String<64>,
    location: heapless::String<32>,
    last_weather_data: Option<WeatherData>,
}

impl<T: NetworkDriver> WeatherService<T> {
    pub fn new(network: T, temperature_celsius: bool) -> Self {
        Self {
            network,
            temperature_celsius,
            api_key: heapless::String::new(),
            location: heapless::String::new(),
            last_weather_data: None,
        }
    }

    pub async fn get_weather(&self) -> Result<WeatherData> {
        // 首先检查网络连接
        if !self.network.is_connected().await {
            warn!("Network not available, using cached or default weather data");
            return self.get_fallback_weather().await;
        }

        // 尝试从API获取天气数据
        match self.fetch_weather_from_api().await {
            Ok(weather) => {
                debug!("Successfully fetched weather data from API");
                Ok(weather)
            }
            Err(e) => {
                warn!("Failed to fetch weather from API: {}, using fallback", e);
                self.get_fallback_weather().await
            }
        }
    }

    pub fn set_api_key(&mut self, api_key: &str) -> Result<()> {
        self.api_key = heapless::String::from_str(api_key)
            .map_err(|_| AppError::ConfigError("API key too long"))?;
        Ok(())
    }

    pub fn set_location(&mut self, location: &str) -> Result<()> {
        self.location = heapless::String::from_str(location)
            .map_err(|_| AppError::ConfigError("Location string too long"))?;
        Ok(())
    }

    pub fn set_temperature_celsius(&mut self, enabled: bool) {
        self.temperature_celsius = enabled;
    }

    pub fn has_valid_data(&self) -> bool {
        self.last_weather_data.is_some()
    }

    async fn fetch_weather_from_api(&self) -> Result<WeatherData> {
        // 检查是否有必要的配置
        if self.api_key.is_empty() || self.location.is_empty() {
            return Err(AppError::ConfigError("Weather API key or location not set"));
        }

        // 构建API请求URL（和风天气）
        let url = self.build_api_url();

        // 发送HTTP请求（这里需要具体的HTTP客户端实现）
        // 简化实现，返回模拟数据
        debug!("Fetching weather from API for location: {}", self.location);

        // 模拟API响应
        let weather_data = WeatherData {
            icon: WeatherIcon::Cloudy,
            temp_current: 23,
            temp_high: 28,
            temp_low: 18,
            humidity: 65,
            wind_direction: WindDirection::East,
            wind_speed: 3,
            air_quality: AirQuality::Good,
        };

        Ok(weather_data)
    }

    async fn get_fallback_weather(&self) -> Result<WeatherData> {
        // 返回缓存的天气数据或默认数据
        if let Some(ref weather) = self.last_weather_data {
            debug!("Using cached weather data");
            Ok(weather.clone())
        } else {
            debug!("Using default weather data");
            Ok(WeatherData::default())
        }
    }

    fn build_api_url(&self) -> heapless::String<256> {
        // 构建和风天气API URL
        // 实际实现应该根据和风天气API文档构建正确的URL
        let mut url = heapless::String::new();
        let _ = write!(
            url,
            "https://devapi.qweather.com/v7/weather/now?key={}&location={}",
            self.api_key, self.location
        );
        url
    }

    // 温度单位转换
    fn convert_temperature(&self, temp: i8) -> i8 {
        if self.temperature_celsius {
            temp
        } else {
            // 摄氏度转华氏度
            (temp * 9 / 5) + 32
        }
    }
}
