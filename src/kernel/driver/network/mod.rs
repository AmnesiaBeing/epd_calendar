// src/driver/network/mod.rs

//! 网络驱动模块
//!
//! 提供网络连接抽象层，支持不同平台的网络驱动实现
//!
//! ## 功能
//! - 定义统一的网络驱动接口 `NetworkDriver`
//! - 支持ESP32和Linux平台的网络实现
//! - 提供网络连接状态管理

use embassy_executor::Spawner;

use crate::{common::error::Result, platform::Platform};

use embedded_svc::wifi::asynch::Wifi;

// ========== NetworkDriver Trait 定义与实现 ==========
/// 网络驱动通用 Trait
pub trait NetworkDriver: Wifi {
    type P: Platform;
    /// 创建驱动实例（非阻塞）
    ///
    /// # 参数
    /// - `peripherals`: 平台外设引用
    /// - `spawner`: 任务 spawner 引用
    ///
    /// # 返回
    /// - `Result<Self>`: 成功返回驱动实例，失败返回错误
    async fn create(
        peripherals: &mut <Self::P as Platform>::Peripherals,
        spawner: &Spawner,
    ) -> Result<Self>
    where
        Self: Sized;

    fn get_stack(&self) -> Option<&embassy_net::Stack>;
}

// 默认网络驱动选择
#[cfg(any(feature = "simulator", feature = "tspi"))]
mod linux;
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub use linux::LinuxNetworkDriver as DefaultNetworkDriver;

#[cfg(feature = "esp32")]
mod esp32;

#[cfg(feature = "esp32")]
pub use esp32::Esp32NetworkDriver as DefaultNetworkDriver;
