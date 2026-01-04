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

use crate::common::error::Result;

/// 网络驱动接口定义
///
/// 提供网络初始化、连接、状态查询等基本功能
pub trait NetworkDriver {
    /// 初始化网络栈
    ///
    /// # 参数
    /// - `spawner`: 异步任务生成器
    ///
    /// # 返回值
    /// - `Result<()>`: 初始化结果
    async fn initialize(&mut self, spawner: &Spawner) -> Result<()>;

    /// 建立网络连接
    ///
    /// # 返回值
    /// - `Result<()>`: 连接结果
    async fn connect(&mut self, ssid: &str, password: &str) -> Result<()>;

    /// 断开网络连接
    ///
    /// # 返回值
    /// - `Result<()>`: 断开结果
    async fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }

    /// 启动AP模式
    ///
    /// # 参数
    /// - `ssid`: AP名称
    /// - `password`: AP密码（可选）
    ///
    /// # 返回值
    /// - `Result<()>`: 启动结果
    async fn start_ap(&mut self, ssid: &str, password: Option<&str>) -> Result<()> {
        Ok(())
    }

    /// 停止AP模式
    ///
    /// # 返回值
    /// - `Result<()>`: 停止结果
    async fn stop_ap(&mut self) -> Result<()> {
        Ok(())
    }

    /// 检查网络连接状态
    ///
    /// # 返回值
    /// - `bool`: 是否已连接
    fn is_connected(&self) -> bool;

    /// 获取网络栈实例
    ///
    /// # 返回值
    /// - `Option<&embassy_net::Stack<'_>>`: 网络栈引用
    fn get_stack(&self) -> Option<&embassy_net::Stack<'_>>;
}

// 默认网络驱动选择
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
mod linux;
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub use linux::LinuxNetworkDriver as DefaultNetworkDriver;

#[cfg(feature = "embedded_esp")]
mod esp;

#[cfg(feature = "embedded_esp")]
pub use esp::EspNetworkDriver as DefaultNetworkDriver;
