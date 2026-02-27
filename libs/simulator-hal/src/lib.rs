//! Simulator HAL - Hardware Abstraction Layer for simulator target
//!
//! This library provides simulated implementations of:
//! - RTC (Real-Time Clock)
//! - WDT (Watchdog Timer)
//! - BLE (Bluetooth Low Energy)
//! - WiFi
//! - HTTP Control Server

pub mod ble;
pub mod http_server;
pub mod rtc;
pub mod wdt;
pub mod wifi;

pub use ble::SimulatedBle;
pub use http_server::HttpServer;
pub use rtc::SimulatedRtc;
pub use wdt::SimulatedWdt;
pub use wifi::SimulatedWifi;
