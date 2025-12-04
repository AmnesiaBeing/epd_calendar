// src/common/error.rs
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AppError {
    // ===== 初始化错误 =====
    #[error("Main initialization failed")]
    MainInit,

    // ===== 网络相关错误 =====
    #[error("Network stack initialization failed")]
    NetworkStackInitFailed,

    #[error("Network stack not initialized")]
    NetworkStackNotInitialized,

    #[error("Network connection failed")]
    NetworkError,

    #[error("WiFi connection failed")]
    WifiConnectionFailed,

    #[error("DHCP failed")]
    DhcpFailed,

    #[error("Socket operation failed")]
    SocketError,

    #[error("DNS resolution failed")]
    DnsResolutionFailed,

    #[error("DNS error")]
    DnsError,

    #[error("HTTPS request failed")]
    HttpsRequestFailed,

    #[error("TLS handshake failed")]
    TlsHandshakeFailed,

    // ===== 显示相关错误 =====
    #[error("Display initialization failed")]
    DisplayInit,

    #[error("Display update failed")]
    DisplayUpdateFailed,

    #[error("Display sleep failed")]
    DisplaySleepFailed,

    #[error("Display full refresh failed")]
    DisplayFullRefreshFailed,

    #[error("Display partial refresh failed")]
    DisplayPartialRefreshFailed,

    #[error("Rendering failed")]
    RenderingFailed,

    // ===== 时间相关错误 =====
    #[error("Time service error")]
    TimeError,

    #[error("SNTP time synchronization failed")]
    SntpSyncFailed,

    #[error("NTP packet invalid")]
    NtpPacketInvalid,

    #[error("RTC update failed")]
    RtcUpdateFailed,

    // ===== 配置相关错误 =====
    #[error("Configuration error: {0}")]
    ConfigError(&'static str),

    #[error("Configuration invalid")]
    ConfigInvalid,

    #[error("Configuration serialization error")]
    ConfigSerializationError,

    #[error("Configuration deserialization error")]
    ConfigDeserializationError,

    #[error("Configuration too large")]
    ConfigTooLarge,

    // ===== 存储相关错误 =====
    #[error("Storage error")]
    StorageError,

    // ===== 其他错误 =====
    #[error("Weather API error")]
    WeatherApiError,

    #[error("Quote error")]
    QuoteError,
}

pub type Result<T> = core::result::Result<T, AppError>;
