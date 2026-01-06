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
use embassy_net::Stack;

use crate::{common::error::Result, platform::Platform};

// ========== 核心枚举定义 ==========
/// 网络工作模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Sleeping,  // 休眠（默认，射频关闭）
    StaActive, // STA 模式（连接 WiFi）
    ApActive,  // AP 模式（配网专用）
}

// ========== RAII 网络锁（核心） ==========
/// 网络栈使用锁（RAII）
/// 获取时自动唤醒网络，释放时自动减少计数并启动休眠延迟
pub struct StackGuard<'a> {
    driver: &'a DefaultNetworkDriver,
    stack: &'a Stack<'static>,
}

impl<'a> StackGuard<'a> {
    /// 获取 embassy-net 原生 Stack 引用（上层直接使用）
    pub fn stack(&self) -> &'a Stack<'static> {
        self.stack
    }
}

impl<'a> Drop for StackGuard<'a> {
    fn drop(&mut self) {
        // 释放时计数 -1
        let prev_count = self
            .driver
            .usage_count
            .fetch_sub(1, core::sync::atomic::Ordering::AcqRel);
        log::debug!(
            "Network stack released, usage count: {} -> {}",
            prev_count,
            prev_count - 1
        );

        // 最后一个使用者释放，启动休眠延迟
        if prev_count == 1 {
            self.driver.start_sleep_delay();
        }
    }
}

// ========== NetworkDriver Trait 定义与实现 ==========
/// 网络驱动通用 Trait
pub trait NetworkDriver {
    type P: Platform;
    /// 创建驱动实例（非阻塞）
    fn new(peripherals: &<Self::P as Platform>::Peripherals, spawner: &Spawner) -> Result<Self>
    where
        Self: Sized;

    /// 获取网络栈锁（RAII）
    async fn acquire_stack(&self) -> Result<StackGuard<'_>>;

    /// 启动 AP 配网模式
    async fn start_ap_for_config(&self) -> Result<()>;

    /// 完成 AP 配网
    async fn finish_ap_config(&self, ssid: &str, password: &str) -> Result<()>;

    /// 检查当前网络模式
    async fn current_mode(&self) -> NetworkMode;

    /// 检查是否已初始化
    fn is_initialized(&self) -> bool;
}

// 默认网络驱动选择
#[cfg(any(feature = "simulator", feature = "tspi"))]
mod linux;
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub use linux::LinuxNetworkDriver as DefaultNetworkDriver;

#[cfg(feature = "esp32")]
mod esp32;

#[cfg(feature = "esp32")]
pub use esp32::EspNetworkDriver as DefaultNetworkDriver;
