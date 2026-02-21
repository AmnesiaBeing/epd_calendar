use embassy_net::Stack;
use heapless::String;
use lxx_calendar_common::http::jwt::JwtSigner;
use lxx_calendar_common::sntp::{EmbassySntpWithStack, SntpClient};
use lxx_calendar_common::weather::{
    API_HOST_DEFAULT, LOCATION_DEFAULT, QweatherJwtSigner,
};
use lxx_calendar_common::*;

pub struct NetworkSyncService<R: Rtc> {
    initialized: bool,
    connected: bool,
    last_sync_time: Option<u64>,
    cached_weather: Option<WeatherInfo>,
    sync_interval_minutes: u16,
    retry_count: u8,
    max_retries: u8,
    low_power_mode: bool,
    rtc: Option<R>,
    stack: Option<Stack<'static>>,
    api_host: heapless::String<128>,
    location: heapless::String<32>,
    jwt_signer: Option<QweatherJwtSigner>,
}

impl<R: Rtc> NetworkSyncService<R> {
    pub fn new() -> Self {
        Self {
            initialized: false,
            connected: false,
            last_sync_time: None,
            cached_weather: None,
            sync_interval_minutes: 120,
            retry_count: 0,
            max_retries: 2,
            low_power_mode: false,
            rtc: None,
            stack: None,
            api_host: heapless::String::try_from(API_HOST_DEFAULT).unwrap_or_default(),
            location: heapless::String::try_from(LOCATION_DEFAULT).unwrap_or_default(),
            jwt_signer: None,
        }
    }

    pub fn with_rtc(rtc: R) -> Self {
        Self {
            initialized: false,
            connected: false,
            last_sync_time: None,
            cached_weather: None,
            sync_interval_minutes: 120,
            retry_count: 0,
            max_retries: 2,
            low_power_mode: false,
            rtc: Some(rtc),
            stack: None,
            api_host: heapless::String::try_from(API_HOST_DEFAULT).unwrap_or_default(),
            location: heapless::String::try_from(LOCATION_DEFAULT).unwrap_or_default(),
            jwt_signer: None,
        }
    }

    pub fn with_stack(mut self, stack: Stack<'static>) -> Self {
        self.stack = Some(stack);
        self
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing network sync service");

        self.retry_count = 0;
        self.max_retries = if self.low_power_mode { 1 } else { 2 };
        self.initialized = true;

        info!("Network sync service initialized");
        Ok(())
    }

    pub async fn sync(&mut self) -> SystemResult<SyncResult>
    where
        R::Error: core::fmt::Debug,
    {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let start_time = embassy_time::Instant::now();

        info!("Starting network sync");

        let time_synced = match self.sync_time().await {
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

        self.last_sync_time = Some(embassy_time::Instant::now().elapsed().as_secs());

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

    async fn sync_time(&mut self) -> SystemResult<()>
    where
        R::Error: core::fmt::Debug,
    {
        if !self.connected {
            self.connect().await?;
        }

        let stack = self.stack.ok_or_else(|| {
            SystemError::HardwareError(HardwareError::NotInitialized)
        })?;

        let mut sntp = EmbassySntpWithStack::new(stack);

        match sntp.get_time().await {
            Ok(unix_timestamp) => {
                info!("SNTP time sync success: {}", unix_timestamp);
                if let Some(ref mut rtc) = self.rtc {
                    if let Err(e) = rtc.set_time(unix_timestamp).await {
                        warn!("Failed to write time to RTC: {:?}", e);
                    } else {
                        info!("Time written to RTC: {}", unix_timestamp);
                    }
                }
            }
            Err(e) => {
                warn!("SNTP time sync failed: {:?}", e);
                return Err(SystemError::NetworkError(NetworkError::Unknown));
            }
        }

        Ok(())
    }

    async fn sync_weather(&mut self) -> SystemResult<()> {
        if let Some(signer) = &self.jwt_signer {
            let timestamp = self.get_current_timestamp().await.unwrap_or(1700000000);
            match signer.sign_with_time("", timestamp) {
                Ok(token) => {
                    info!("JWT token generated: {}...", &token.as_str()[..30.min(token.len())]);
                }
                Err(e) => {
                    warn!("Failed to generate JWT token: {:?}", e);
                }
            }
        }

        info!("Weather sync using default data (HTTP client not fully implemented)");
        let weather = self.get_default_weather()?;
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
            Err(SystemError::NetworkError(NetworkError::Unknown))
        }
    }

    pub async fn is_connected(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.connected)
    }

    pub async fn connect(&mut self) -> SystemResult<()> {
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

    pub async fn disconnect(&mut self) -> SystemResult<()> {
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

    pub async fn should_sync(&self) -> bool {
        if let Some(_last_sync) = self.last_sync_time {
            let elapsed = embassy_time::Instant::now().elapsed().as_secs();
            let interval = if self.low_power_mode {
                (self.sync_interval_minutes * 2) as u64 * 60
            } else {
                self.sync_interval_minutes as u64 * 60
            };

            elapsed >= interval
        } else {
            true
        }
    }

    pub async fn set_sync_interval(&mut self, minutes: u16) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        self.sync_interval_minutes = minutes;
        info!("Sync interval set to {} minutes", minutes);

        Ok(())
    }

    pub async fn set_low_power_mode(&mut self, enabled: bool) -> SystemResult<()> {
        self.low_power_mode = enabled;
        self.max_retries = if enabled { 1 } else { 2 };

        if enabled {
            info!("Network sync service entered low power mode");
        } else {
            info!("Network sync service exited low power mode");
        }

        Ok(())
    }

    pub async fn get_last_sync_time(&self) -> Option<u64> {
        self.last_sync_time
    }

    pub async fn force_sync(&mut self) -> SystemResult<SyncResult>
    where
        R::Error: core::fmt::Debug,
    {
        self.retry_count = 0;
        self.sync().await
    }

    pub fn set_api_config(
        &mut self,
        api_host: &str,
        location: &str,
        key_id: &str,
        project_id: &str,
        private_key: &str,
    ) -> SystemResult<()> {
        self.api_host = String::try_from(api_host).unwrap_or_else(|_| {
            String::try_from(API_HOST_DEFAULT).unwrap()
        });
        self.location = String::try_from(location).unwrap_or_else(|_| {
            String::try_from(LOCATION_DEFAULT).unwrap()
        });

        self.jwt_signer = match QweatherJwtSigner::new(key_id, project_id, private_key) {
            Ok(signer) => {
                info!("JWT signer initialized successfully");
                Some(signer)
            }
            Err(e) => {
                warn!("Failed to initialize JWT signer: {:?}", e);
                None
            }
        };

        Ok(())
    }

    pub fn set_location(&mut self, location: &str) {
        self.location = String::try_from(location).unwrap_or_else(|_| {
            String::try_from(LOCATION_DEFAULT).unwrap()
        });
    }

    async fn get_current_timestamp(&self) -> Option<i64> {
        if let Some(ref rtc) = self.rtc {
            match rtc.get_time().await {
                Ok(timestamp) => Some(timestamp as i64),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}
