// src/driver/network.rs
use embassy_executor::Spawner;

use crate::common::error::Result;

pub trait NetworkDriver {
    async fn initialize(&mut self, spawner: &Spawner) -> Result<()>;
    async fn connect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
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
