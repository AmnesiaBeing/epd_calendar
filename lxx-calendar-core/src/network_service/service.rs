use lxx_calendar_common as lxx_common;
use lxx_common::{SystemResult, SystemError};

pub struct NetworkService {
    initialized: bool,
}

impl NetworkService {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        lxx_common::info!("Initializing network service");
        self.initialized = true;
        Ok(())
    }

    pub async fn sync(&mut self) -> SystemResult<lxx_common::SyncResult> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        lxx_common::info!("Syncing network");
        Ok(lxx_common::SyncResult {
            time_synced: true,
            weather_synced: true,
            sync_duration: embassy_time::Duration::from_secs(5),
        })
    }

    pub async fn get_weather(&self) -> SystemResult<lxx_common::WeatherInfo> {
        if !self.initialized {
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        Ok(lxx_common::WeatherInfo {
            location: heapless::String::try_from("上海").unwrap_or_default(),
            current: lxx_common::CurrentWeather {
                temp: 220,
                feels_like: 218,
                humidity: 65,
                condition: lxx_common::WeatherCondition::Cloudy,
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
            return Err(lxx_common::SystemError::HardwareError(lxx_common::HardwareError::NotInitialized));
        }
        Ok(false)
    }
}