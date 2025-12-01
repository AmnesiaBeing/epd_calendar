// src/service/mod.rs
pub mod config_service;
pub mod quote_service;
pub mod time_service;
pub mod weather_service;

pub use config_service::ConfigService;
pub use quote_service::QuoteService;
pub use time_service::TimeService;
pub use weather_service::WeatherService;
