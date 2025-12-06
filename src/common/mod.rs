// src/common/mod.rs

/// 公共模块定义
///
/// 本模块定义了EPD日历系统的公共类型、常量和模块导出
/// 包含配置、错误处理、系统状态、天气数据等公共组件
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

/// 全局互斥锁类型别名
///
/// 根据目标平台选择不同的互斥锁实现
/// - 模拟器和嵌入式Linux：使用ThreadModeRawMutex
/// - 嵌入式ESP平台：使用ESP的RawMutex
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type GlobalMutex<T> = Mutex<ThreadModeRawMutex, T>;
#[cfg(feature = "embedded_esp")]
pub type GlobalMutex<T> = Mutex<RawMutex, T>;

/// 全局通道类型别名
///
/// 根据目标平台选择不同的通道实现
/// - 模拟器和嵌入式Linux：使用ThreadModeRawMutex
/// - 嵌入式ESP平台：使用ESP的RawMutex
/// 通道容量固定为32个元素
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type GlobalChannel<T> = Channel<ThreadModeRawMutex, T, 32>;
#[cfg(feature = "embedded_esp")]
pub type GlobalChannel<T> = Channel<RawMutex, T, 32>;
