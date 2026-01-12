use lxx_calendar_common::*;

pub struct NetworkService {
    initialized: bool,
}

impl NetworkService {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing network service");
        self.initialized = true;
        Ok(())
    }

    pub async fn sync(&mut self) -> SystemResult<SyncResult> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        info!("Syncing network");
        Ok(SyncResult {
            time_synced: true,
            weather_synced: true,
            sync_duration: embassy_time::Duration::from_secs(5),
        })
    }

    pub async fn get_weather(&self) -> SystemResult<WeatherInfo> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(WeatherInfo {
            location: heapless::String::try_from("上海").unwrap_or_default(),
            current: CurrentWeather {
                temp: 220,
                feels_like: 218,
                humidity: 65,
                condition: WeatherCondition::Cloudy,
                wind_speed: 10,
                wind_direction: 180,
                visibility: 10,
                pressure: 1013,
                update_time: 1705315200,
            },
            forecast: heapless::Vec::new(),
            last_update: 1705315200,
        })
    }

    pub async fn is_connected(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(false)
    }
}
