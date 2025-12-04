pub mod config;
pub mod error;
pub mod system_state;
pub mod weather;

pub use system_state::SystemState;

pub use crate::assets::generated_hitokoto_data::Hitokoto;
pub use system_state::BatteryLevel;
pub use system_state::ChargingStatus;
pub use system_state::DateData;
pub use system_state::NetworkStatus;
pub use system_state::TimeData;
pub use system_state::WeatherData;

use embassy_sync::{channel::Channel, mutex::Mutex};

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
#[cfg(feature = "embedded_esp")]
use esp_sync::RawMutex;

// 按照不同平台使用不同的锁
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type GlobalMutex<T> = Mutex<ThreadModeRawMutex, T>;
#[cfg(feature = "embedded_esp")]
pub type GlobalMutex<T> = Mutex<RawMutex, T>;

// 按照不同平台使用不同的通道
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type GlobalChannel<T> = Channel<ThreadModeRawMutex, T, 32>;
#[cfg(feature = "embedded_esp")]
pub type GlobalChannel<T> = Channel<RawMutex, T, 32>;
