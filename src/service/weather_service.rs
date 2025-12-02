// src/service/weather_service.rs
use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::common::weather::{QWeatherResponse, QWeatherStatusCode, WeatherData};
use crate::driver::network::{DefaultNetworkDriver, NetworkDriver};
use alloc::format;

pub struct WeatherService {
    network: &'static GlobalMutex<DefaultNetworkDriver>,
    location_id: heapless::String<20>,
    last_weather_data: Option<WeatherData>,
}

const API_HOST: &str = "devapi.qweather.com";
const API_PATH: &str = "/v7/weather/3d";
const API_KAY: &str = "";

impl WeatherService {
    pub fn new(network: &'static GlobalMutex<DefaultNetworkDriver>) -> Self {
        Self {
            network,
            location_id: heapless::String::new(),
            last_weather_data: None,
        }
    }

    pub async fn get_weather(&mut self) -> Result<WeatherData> {
        // 首先检查网络连接
        if !self.network.lock().await.is_connected() {
            log::warn!("Network not available, using cached or default weather data");
            return Err(AppError::WeatherApiError);
        }

        // 尝试从API获取天气数据
        match self.fetch_weather_from_api().await {
            Ok(weather) => {
                log::debug!("Successfully fetched weather data from API");
                self.last_weather_data = Some(weather.clone());
                Ok(weather)
            }
            Err(e) => {
                log::warn!("Failed to fetch weather from API: {}, using fallback", e);
                return Err(AppError::WeatherApiError);
            }
        }
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
        log::debug!(
            "Fetching weather from API for location: {}",
            self.location_id
        );

        let mut buffer: [u8; 4096] = [0; 4096];

        // let response = self
        //     .network
        //     .lock()
        //     .await
        //     .https_get("devapi.qweather.com", &path, &mut buffer)
        //     .await
        //     .map_err(|e| {
        //         log::warn!("HTTP request failed: {:?}", e);
        //         AppError::NetworkError
        //     })?;

        let response = [0u8; 4096];

        // 解析响应
        self.parse_weather_response(&response)
    }

    /// 解析和风天气API的JSON响应
    fn parse_weather_response(&self, response: &[u8]) -> Result<WeatherData> {
        log::info!("Parsing weather response ({} bytes)", response.len());

        // 反序列化JSON
        let qweather_resp: QWeatherResponse = serde_json::from_slice(response).map_err(|e| {
            log::warn!("JSON parse error: {:?}", e);
            AppError::WeatherApiError
        })?;

        // 2. 校验API状态码
        if qweather_resp.code != QWeatherStatusCode::Success {
            log::warn!("API returned error code: {:?}", qweather_resp.code);
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
}
