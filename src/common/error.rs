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

    #[error("Quote error")]
    QuoteError,

    #[error("WiFi connection failed")]
    WifiConnectionFailed,

    #[error("SNTP time synchronization failed")]
    SntpSyncFailed,

    #[error("HTTPS request failed")]
    HttpsRequestFailed,

    #[error("TLS handshake failed")]
    TlsHandshakeFailed,

    #[error("Network stack initialization failed")]
    NetworkStackInitFailed,

    #[error("DNS resolution failed")]
    DnsResolutionFailed,

    #[error("Socket operation failed")]
    SocketError,

    #[error("NTP packet invalid")]
    NtpPacketInvalid,

    #[error("RTC update failed")]
    RtcUpdateFailed,

    #[error("DHCP failed")]
    DhcpFailed,

    #[error("Network stack not initialized")]
    NetworkStackNotInitialized,

    #[error("Configuration invalid")]
    ConfigInvalid,

    #[error("Configuration serialization error")]
    ConfigSerializationError,

    #[error("Configuration deserialization error")]
    ConfigDeserializationError,

    #[error("Configuration too large")]
    ConfigTooLarge,
}

pub type Result<T> = core::result::Result<T, AppError>;
