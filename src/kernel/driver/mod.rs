// src/driver/mod.rs

/// 驱动模块定义
///
/// 本模块定义了EPD日历系统的硬件驱动接口和实现
/// 包含显示、网络、时间、电源、传感器等硬件驱动组件
pub mod button;
pub mod buzzer;
pub mod display;
pub mod led;
pub mod network;
pub mod ntp_source;
pub mod power;
pub mod sensor;
pub mod storage;
pub mod time_driver;

/// ESP平台下的随机数生成器模块
#[cfg(feature = "esp32")]
pub mod rng;