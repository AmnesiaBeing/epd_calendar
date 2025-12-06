// src/service/mod.rs

//! 服务模块 - 提供应用核心业务逻辑服务
//!
//! 该模块包含配置管理、时间服务、天气服务和名言服务等核心业务逻辑。

pub mod config_service;
pub mod quote_service;
pub mod time_service;
pub mod weather_service;

pub use config_service::ConfigService;
pub use quote_service::QuoteService;
pub use time_service::TimeService;
pub use weather_service::WeatherService;
