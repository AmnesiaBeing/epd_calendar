// src/common/error.rs

/// 应用程序错误定义模块
///
/// 本模块定义了EPD日历系统中所有可能的错误类型，使用thiserror库进行错误处理
/// 错误类型按功能模块分类，便于错误定位和处理
use thiserror::Error;

/// 应用程序错误枚举
///
/// 定义了EPD日历系统中所有可能出现的错误类型，按功能模块分类
#[derive(Error, Debug, Clone)]
pub enum AppError {
    // ===== 资源相关错误 =====
    #[error("Invalid weather icon code")]
    InvalidWeatherIconCode,

    #[error("Configuration initialization failed")]
    ConfigInitFailed,

    #[error("Invalid weather date format")]
    InvalidWeatherDate,

    #[error("Invalid sensor value")]
    InvalidSensorValue,

    #[error("Sensor error")]
    SensorError,

    #[error("String conversion error")]
    ConvertError,

    #[error("Layout load failed")]
    LayoutLoadFailed,

    #[error("Condition evaluation failed")]
    ConditionEvaluationFailed,

    #[error("Layout placeholder error")]
    LayoutPlaceholder,

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

    #[error("Network request failed")]
    NetworkRequestFailed,

    #[error("Network response too large")]
    NetworkResponseTooLarge,

    #[error("Network response invalid")]
    NetworkResponseInvalid,

    #[error("Buffer overflow")]
    BufferOverflow,

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

    // ===== 布局相关错误 =====
    #[error("Layout deserialization failed")]
    LayoutDeserialize,

    #[error("Layout condition parse failed")]
    LayoutConditionParse,

    #[error("Layout placeholder not found")]
    LayoutPlaceholderNotFound,

    #[error("Invalid icon ID")]
    InvalidIconId,

    #[error("Render element failed")]
    RenderElementFailed,

    #[error("Render error")]
    RenderError,

    // ===== 其他错误 =====
    #[error("Weather API error")]
    WeatherApiError,

    #[error("Quote error")]
    QuoteError,

    #[error("Data source not found")]
    DataSourceNotFound,

    #[error("Data source refresh failed")]
    DataSourceRefreshFailed,

    #[error("Invalid field name")]
    InvalidFieldName,

    #[error("Field not found")]
    FieldNotFound,

    #[error("Field already exists")]
    FieldAlreadyExists,

    #[error("Data capacity exceeded")]
    DataCapacityExceeded,

    #[error("Field limit exceeded")]
    FieldLimitExceeded,

    #[error("Lunar calculation error")]
    LunarCalculationError,

    #[error("Cache set failed")]
    CacheSetFailed,

    #[error("Cache miss")]
    CacheMiss,

    #[error("Time driver error")]
    TimeDriverError,

    #[error("Invalid location ID")]
    InvalidLocationId,

    #[error("Invalid weather condition")]
    InvalidWeatherCondition,

    #[error("Field metadata limit exceeded")]
    FieldMetaLimitExceeded,

    #[error("Invalid API URL")]
    InvalidApiUrl,

    #[error("JSON parse failed")]
    JsonParseFailed,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Weather data not found")]
    WeatherDataNotFound,

    #[error("Invalid path format")]
    InvalidPathFormat,

    #[error("Invalid config data")]
    InvalidConfigData,

    #[error("Power driver error")]
    PowerError,

    #[error("Invalid date")]
    InvalidDate,

    #[error("Data source registry full")]
    DataSourceRegistryFull,

    #[error("Data source busy")]
    DataSourceBusy,

    #[error("Icon not found")]
    IconNotFound,

    #[error("Render failed")]
    RenderFailed,
}

/// 应用程序结果类型别名
///
/// 简化错误处理，使用AppError作为错误类型
pub type Result<T> = core::result::Result<T, AppError>;
