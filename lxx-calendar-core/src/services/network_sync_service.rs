use embassy_net::Stack;
use heapless::String;
use lxx_calendar_common::sntp::{EmbassySntpWithStack, SntpClient};
use lxx_calendar_common::weather::OpenMeteoResponse;
use lxx_calendar_common::weather::openmeteo_converter::convert_openmeteo_response;
use lxx_calendar_common::{
    error, info,
    traits::{Rtc, WifiController},
    types::error::{HardwareError, NetworkError, SystemError, SystemResult},
    types::weather::{CurrentWeather, ForecastDay, WeatherCondition, WeatherInfo},
    warn,
};

use crate::services::http_client::{HttpClientImpl, HttpError, RequestImpl};
use crate::services::time_service::TimeService;
use lxx_calendar_common::http::http::{HttpClient, HttpResponse};

extern crate alloc;
use alloc::format;

pub struct SyncResult {
    pub time_synced: bool,
    pub weather_synced: bool,
    pub quote_updated: bool,
    pub sync_duration: u64,
}

const TLS_RX_BUFFER_SIZE: usize = 16384;
const TLS_TX_BUFFER_SIZE: usize = 16384;

pub struct NetworkSyncService {
    initialized: bool,
    connected: bool,
    cached_weather: Option<WeatherInfo>,
    retry_count: u8,
    max_retries: u8,
    stack: Option<Stack<'static>>,
    latitude: f64,
    longitude: f64,
    location_name: heapless::String<32>,
    wifi_config: Option<(heapless::String<32>, heapless::String<64>)>,
    sync_in_progress: bool,
}

impl NetworkSyncService {
    pub fn new() -> Self {
        Self {
            initialized: false,
            connected: false,
            cached_weather: None,
            retry_count: 0,
            max_retries: 2,
            stack: None,
            latitude: 0.0,
            longitude: 0.0,
            location_name: heapless::String::new(),
            wifi_config: None,
            sync_in_progress: false,
        }
    }

    pub fn with_stack(mut self, stack: Stack<'static>) -> Self {
        self.stack = Some(stack);
        self
    }

    pub fn set_stack(&mut self, stack: Stack<'static>) {
        info!("Setting network stack for sync service");
        self.stack = Some(stack);
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing network sync service");

        self.retry_count = 0;
        self.initialized = true;

        info!("Network sync service initialized");
        Ok(())
    }

    pub fn save_wifi_config(&mut self, ssid: heapless::String<32>, password: heapless::String<64>) {
        info!("Saving WiFi config: ssid={}", ssid);
        self.wifi_config = Some((ssid, password));
    }

    pub async fn connect_wifi<W: WifiController>(&mut self, wifi: &mut W) -> SystemResult<()> {
        if let Some((ref ssid, ref password)) = self.wifi_config {
            info!("Connecting to WiFi: {}", ssid);
            match wifi.connect_sta(ssid, password).await {
                Ok(_) => {
                    self.connected = true;
                    info!("WiFi connected successfully");
                    Ok(())
                }
                Err(_e) => {
                    error!("WiFi connection failed");
                    return Err(SystemError::NetworkError(NetworkError::Unknown));
                }
            }
        } else {
            Err(SystemError::HardwareError(HardwareError::InvalidParameter))
        }
    }

    pub async fn sync<'a, R: Rtc>(
        &'a mut self,
        time_service: &'a mut TimeService<R>,
    ) -> SystemResult<SyncResult> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let start_time = embassy_time::Instant::now();

        info!("Starting network sync");

        let time_synced = match self.sync_time(time_service).await {
            Ok(_) => {
                info!("Time synchronized successfully");
                true
            }
            Err(_e) => {
                error!("Time sync failed");
                return Err(SystemError::NetworkError(NetworkError::Unknown));
            }
        };

        let weather_synced = match self.sync_weather().await {
            Ok(_) => {
                info!("Weather synchronized successfully");
                true
            }
            Err(_e) => {
                error!("Weather sync failed");
                return Err(SystemError::NetworkError(NetworkError::Unknown));
            }
        };

        let sync_duration = start_time.elapsed().as_secs();

        if !time_synced || !weather_synced {
            self.retry_count += 1;
            if self.retry_count < self.max_retries {
                info!(
                    "Sync failed, will retry (attempt {}/{})",
                    self.retry_count + 1,
                    self.max_retries
                );
            }
        } else {
            self.retry_count = 0;
        }

        self.disconnect().await?;

        Ok(SyncResult {
            time_synced,
            weather_synced,
            quote_updated: false,
            sync_duration,
        })
    }

    async fn sync_time<R: Rtc>(&mut self, time_service: &mut TimeService<R>) -> SystemResult<()> {
        if !self.connected {
            self.connect().await?;
        }

        // Get stack for SNTP
        let stack = self
            .stack
            .as_ref()
            .ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;

        let mut sntp = EmbassySntpWithStack::new(*stack);

        match sntp.get_time().await {
            Ok(unix_timestamp) => {
                info!("SNTP time sync success: {}", unix_timestamp);
                if let Err(e) = time_service.set_time(unix_timestamp as u64).await {
                    warn!("Failed to set time: {:?}", e);
                } else {
                    info!("Time set: {}", unix_timestamp);
                }
            }
            Err(_) => {
                warn!("SNTP time sync failed");
                return Err(SystemError::NetworkError(NetworkError::Unknown));
            }
        }

        Ok(())
    }

    async fn sync_weather(&mut self) -> SystemResult<()> {
        if !self.connected {
            self.connect().await?;
        }

        // Check if coordinates are configured
        if self.latitude == 0.0 || self.longitude == 0.0 {
            warn!("Latitude/longitude not set, using default weather");
            let weather = self.get_default_weather()?;
            self.cached_weather = Some(weather);
            return Ok(());
        }

        // Get stack for HTTP
        let stack = self
            .stack
            .as_ref()
            .ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;

        // 创建 TLS 缓冲区
        let mut tls_rx_buf: [u8; TLS_RX_BUFFER_SIZE] = [0; TLS_RX_BUFFER_SIZE];
        let mut tls_tx_buf: [u8; TLS_TX_BUFFER_SIZE] = [0; TLS_TX_BUFFER_SIZE];
        let mut http_client = HttpClientImpl::new(*stack, &mut tls_rx_buf, &mut tls_tx_buf);

        // 使用 Open-Meteo API (HTTP)
        // 注意：Open-Meteo 支持 HTTP 和 HTTPS，嵌入式设备可先用 HTTP 测试
        let url = format!(
            "http://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m&daily=weather_code,temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=3",
            self.latitude, self.longitude
        );

        info!("Requesting Open-Meteo API: {}", url);

        // 创建请求
        let request = RequestImpl::new(lxx_calendar_common::http::http::HttpMethod::GET, &url);

        let result = http_client.request(&request).await;

        let (status, body_vec) = match result {
            Ok(response) => {
                let status = response.status();
                let body = response.body();
                let body_vec: heapless::Vec<u8, 16384> = body.iter().copied().collect();
                (status, body_vec)
            }
            Err(e) => {
                warn!("HTTP request failed: {:?}", e);
                return Err(SystemError::NetworkError(NetworkError::Unknown));
            }
        };

        info!("Open-Meteo API response status: {}", status);

        if status != 200 {
            if let Ok(response_str) = core::str::from_utf8(body_vec.as_slice()) {
                warn!(
                    "Open-Meteo API returned status: {}, response: {}",
                    status, response_str
                );
            } else {
                warn!(
                    "Open-Meteo API returned status: {}, response: (non-UTF8)",
                    status
                );
            }
            return Err(SystemError::NetworkError(NetworkError::Unknown));
        }

        let response_str = core::str::from_utf8(body_vec.as_slice()).map_err(|_| {
            warn!("Failed to parse response as UTF-8");
            SystemError::NetworkError(NetworkError::Unknown)
        })?;

        info!(
            "Open-Meteo API response received, length: {} bytes",
            response_str.len()
        );

        let api_response: OpenMeteoResponse = serde_json::from_str(response_str).map_err(|e| {
            warn!("Failed to parse Open-Meteo JSON: {:?}", e);
            SystemError::NetworkError(NetworkError::Unknown)
        })?;

        let location_name = if self.location_name.is_empty() {
            "未知"
        } else {
            self.location_name.as_str()
        };

        let weather = convert_openmeteo_response(&api_response, location_name);

        self.cached_weather = Some(weather);

        info!("Weather data cached successfully");

        Ok(())
    }

    fn get_default_weather(&self) -> SystemResult<WeatherInfo> {
        Ok(WeatherInfo {
            location: heapless::String::try_from("上海").unwrap_or_default(),
            current: CurrentWeather {
                temp: 22,
                feels_like: 20,
                humidity: 65,
                condition: WeatherCondition::Cloudy,
                wind_speed: 10,
                wind_direction: 180,
                visibility: 10,
                pressure: 1013,
                update_time: embassy_time::Instant::now().elapsed().as_secs() as i64,
            },
            forecast: {
                let mut forecast = heapless::Vec::new();
                forecast
                    .push(ForecastDay {
                        date: 0,
                        high_temp: 25,
                        low_temp: 18,
                        condition: WeatherCondition::Sunny,
                        humidity: 60,
                    })
                    .ok();
                forecast
                    .push(ForecastDay {
                        date: 86400,
                        high_temp: 24,
                        low_temp: 17,
                        condition: WeatherCondition::Cloudy,
                        humidity: 65,
                    })
                    .ok();
                forecast
                    .push(ForecastDay {
                        date: 172800,
                        high_temp: 23,
                        low_temp: 16,
                        condition: WeatherCondition::LightRain,
                        humidity: 80,
                    })
                    .ok();
                forecast
            },
            last_update: embassy_time::Instant::now().elapsed().as_secs() as i64,
        })
    }

    pub async fn get_weather(&self) -> SystemResult<WeatherInfo> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if let Some(ref weather) = self.cached_weather {
            Ok(weather.clone())
        } else {
            Ok(self.get_default_weather()?)
        }
    }

    pub async fn is_connected(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.connected)
    }

    async fn connect(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if self.connected {
            return Ok(());
        }

        info!("Connecting to Wi-Fi");

        self.connected = true;
        info!("Wi-Fi connected");

        Ok(())
    }

    async fn disconnect(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if !self.connected {
            return Ok(());
        }

        info!("Disconnecting from Wi-Fi");

        self.connected = false;

        info!("Wi-Fi disconnected");

        Ok(())
    }

    pub fn set_location(&mut self, latitude: f64, longitude: f64, name: &str) {
        self.latitude = latitude;
        self.longitude = longitude;
        if let Ok(s) = String::try_from(name) {
            self.location_name = s;
        }
        info!("Location set: {}, {} ({})", latitude, longitude, name);
    }
}
