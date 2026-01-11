// src/driver/time_source/mod.rs

//! 时间源驱动模块
//!
//! 提供系统时间获取和设置的抽象层，支持不同平台的时间源实现
//!
//! ## 功能
//! - 定义统一的时间源接口 `TimeSource`
//! - 支持ESP32（硬件RTC）和Linux（模拟RTC）平台
//! - 提供时间获取、设置和时区处理功能
//!
//! ## 时间逻辑说明
//! - ESP32内部使用两个u32的RTC寄存器存储时间
//! - 模拟器使用单个u64存储时间戳
//! - 存储的时间类型均为`Timestamp`（u64），时区信息单独处理
//!
//! ## SNTP和时间源关系
//! - ESP32通过SNTP更新时间后调用`set_current_time_us`写入寄存器
//! - 模拟器通过SNTP更新时间后调用`update_timestamp`方法更新时间戳

use jiff::Timestamp;

use crate::{common::error::Result, platform::Platform};

/// 时间源驱动接口定义
///
/// 提供系统时间的获取和设置功能
pub trait TimeDriver {
    type P: Platform;
    /// 创建新的时间源实例
    ///
    /// # 参数
    /// - `peripherals`: 平台特定的外设引用
    ///
    /// # 返回值
    /// - `Self`: 时间源实例
    fn create(peripherals: &mut <Self::P as Platform>::Peripherals) -> Result<Self>
    where
        Self: Sized;

    /// 获取当前时间（UTC时间戳）
    ///
    /// # 返回值
    /// - `Result<Timestamp>`: 当前时间戳或错误
    fn get_time(&self) -> Result<Timestamp>;

    /// 设置新时间
    ///
    /// # 参数
    /// - `new_time`: 新的时间戳
    ///
    /// # 返回值
    /// - `Result<()>`: 设置结果
    fn set_time(&mut self, new_time: Timestamp) -> Result<()>;
}

// 默认时间源选择
#[cfg(any(feature = "simulator", feature = "tspi"))]
mod simulator;

#[cfg(feature = "esp32c6")]
mod esp32c6;

#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type DefaultTimeDriver = simulator::SimulatedRtc;

#[cfg(feature = "esp32c6")]
pub type DefaultTimeDriver = esp32c6::RtcTimeDriver;
