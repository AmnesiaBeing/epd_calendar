use lxx_calendar_common as lxxcc;
use lxxcc::{SystemResult, SystemError};

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
        lxxcc::info!("Initializing network service");
        self.initialized = true;
        Ok(())
    }

    pub async fn sync(&mut self) -> SystemResult<lxxcc::SyncResult> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        lxxcc::info!("Syncing network");
        Ok(lxxcc::SyncResult {
            time_synced: true,
            weather_synced: true,
            sync_duration: embassy_time::Duration::from_secs(5),
        })
    }

    pub async fn get_weather(&self) -> SystemResult<lxxcc::WeatherInfo> {
        if !self.initialized {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        Ok(lxxcc::WeatherInfo {
            location: heapless::String::try_from("上海").unwrap_or_default(),
            current: lxxcc::CurrentWeather {
                temp: 220,
                feels_like: 218,
                humidity: 65,
                condition: lxxcc::WeatherCondition::Cloudy,
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
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::NotInitialized));
        }
        Ok(false)
    }
}