// src/service/mod.rs
pub mod config_service;
pub mod quote_service;
pub mod time_service;
pub mod weather_service;

pub use config_service::config_service;
pub use quote_service::quote_service;
pub use time_service::time_service;
pub use weather_service::weather_service;
