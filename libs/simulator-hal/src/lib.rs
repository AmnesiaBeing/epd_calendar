//! Simulator HAL - Hardware Abstraction Layer for simulator target
//! 
//! This library provides simulated implementations of:
//! - RTC (Real-Time Clock)
//! - WDT (Watchdog Timer)
//! - BLE (Bluetooth Low Energy)
//! - WiFi
//! - HTTP Control Server

pub mod config;
pub mod ble;
pub mod wifi;
pub mod http_server;
pub mod state;

pub use config::SimulatorConfig;
pub use ble::SimulatedBle;
pub use wifi::SimulatedWifi;
pub use http_server::HttpServer;
pub use state::SimulatorState;
