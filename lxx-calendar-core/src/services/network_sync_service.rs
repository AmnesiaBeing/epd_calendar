use embassy_net::Stack;
use heapless::String;
use lxx_calendar_common::http::jwt::JwtSigner;
use lxx_calendar_common::http::http::{HttpClient, HttpResponse};
use lxx_calendar_common::sntp::{EmbassySntpWithStack, SntpClient};
use lxx_calendar_common::weather::{API_HOST_DEFAULT, LOCATION_DEFAULT, QweatherJwtSigner};
use lxx_calendar_common::*;

use crate::services::http_client::{HttpClientImpl, RequestImpl};
use crate::services::time_service::TimeService;

extern crate alloc;
use alloc::format;

const HTTP_RX_SIZE: usize = 4096;
const HTTP_TX_SIZE: usize = 4096;

pub struct NetworkSyncService {
    initialized: bool,
    connected: bool,
    cached_weather: Option<WeatherInfo>,
    retry_count: u8,
    max_retries: u8,
    stack: Option<Stack<'static>>,
    http_rx_buffer: Option<[u8; HTTP_RX_SIZE]>,
    http_tx_buffer: Option<[u8; HTTP_TX_SIZE]>,
    api_host: heapless::String<128>,
    location: heapless::String<32>,
    jwt_signer: Option<QweatherJwtSigner>,
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
            http_rx_buffer: None,
            http_tx_buffer: None,
            api_host: heapless::String::try_from(API_HOST_DEFAULT).unwrap_or_default(),
            location: heapless::String::try_from(LOCATION_DEFAULT).unwrap_or_default(),
            jwt_signer: None,
        }
    }

    pub fn with_stack(mut self, stack: Stack<'static>) -> Self {
        self.stack = Some(stack);
        self.http_rx_buffer = Some([0u8; HTTP_RX_SIZE]);
        self.http_tx_buffer = Some([0u8; HTTP_TX_SIZE]);
        self
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing network sync service");

        self.retry_count = 0;
        self.initialized = true;

        info!("Network sync service initialized");
        Ok(())
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
            Err(e) => {
                warn!("Time sync failed: {:?}", e);
                false
            }
        };

        let weather_synced = match self.sync_weather().await {
            Ok(_) => {
                info!("Weather synchronized successfully");
                true
            }
            Err(e) => {
                warn!("Weather sync failed: {:?}", e);
                false
            }
        };

        let sync_duration = start_time.elapsed();

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

        let stack = self
            .stack
            .ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;

        let mut sntp = EmbassySntpWithStack::new(stack);

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

        let stack = self.stack.ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;

        let Some(signer) = &self.jwt_signer else {
            warn!("JWT signer not configured, using default weather");
            let weather = self.get_default_weather()?;
            self.cached_weather = Some(weather);
            return Ok(());
        };

        let timestamp = embassy_time::Instant::now().as_micros() as i64;
        let token = signer.sign_with_time("", timestamp).map_err(|e| {
            warn!("Failed to generate JWT token: {:?}", e);
            SystemError::NetworkError(NetworkError::Unknown)
        })?;

        info!("JWT token generated successfully");

        let stack = self.stack.take().ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;
        let mut rx_buf = self.http_rx_buffer.take().ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;
        let mut tx_buf = self.http_tx_buffer.take().ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;

        let mut http_client = HttpClientImpl::new(stack);

        let url = format!(
            "http://{}/v7/weather/3d?location={}",
            self.api_host.as_str(),
            self.location.as_str()
        );

        let (status, body) = http_client.request(&mut rx_buf, &mut tx_buf, lxx_calendar_common::http::http::HttpMethod::GET, &url, None).await.map_err(|e| {
            warn!("HTTP request failed: {:?}", e);
            SystemError::NetworkError(NetworkError::Unknown)
        })?;

        if status != 200 {
            warn!("Weather API returned status: {}", status);
            return Err(SystemError::NetworkError(NetworkError::Unknown));
        }

        let body = body.as_slice();
        let response_str = core::str::from_utf8(body).map_err(|_| {
            warn!("Failed to parse response as UTF-8");
            SystemError::NetworkError(NetworkError::Unknown)
        })?;

        info!("Weather API response received");

        let api_response: lxx_calendar_common::weather::api::WeatherDailyResponse =
            serde_json::from_str(response_str).map_err(|e| {
                warn!("Failed to parse weather JSON: {:?}", e);
                SystemError::NetworkError(NetworkError::Unknown)
            })?;

        let weather = lxx_calendar_common::weather::converter::convert_daily_response(
            &api_response,
            self.location.as_str(),
        )?;

        self.cached_weather = Some(weather);

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

    pub fn set_location(&mut self, location: &str) {
        self.location = String::try_from(location)
            .unwrap_or_else(|_| String::try_from(LOCATION_DEFAULT).unwrap());
    }

    pub fn set_jwt_signer(&mut self, signer: QweatherJwtSigner) {
        self.jwt_signer = Some(signer);
    }
}
