// src/common/error.rs
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AppError {
    #[error("Main initialization failed")]
    MainInit,

    #[error("Display initialization failed")]
    DisplayInit,

    #[error("Network connection failed")]
    NetworkError,

    #[error("Storage error")]
    StorageError,

    #[error("Configuration error: {0}")]
    ConfigError(&'static str),

    #[error("Time service error")]
    TimeError,

    #[error("Display update failed")]
    DisplayUpdateFailed,

    #[error("Display sleep failed")]
    DisplaySleepFailed,

    #[error("DNS error")]
    DnsError,

    #[error("Weather API error")]
    WeatherApiError,
}

pub type Result<T> = core::result::Result<T, AppError>;
