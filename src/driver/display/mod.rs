// src/driver/display/mod.rs
use crate::common::error::Result;

#[cfg(feature = "simulator")]
mod simulator;

#[cfg(feature = "embedded_linux")]
mod linux;

#[cfg(feature = "embedded_esp")]
mod esp;

// 条件编译导入
#[cfg(feature = "simulator")]
pub type DefaultDisplayDriver = simulator::SimulatorEpdDriver;

#[cfg(feature = "embedded_linux")]
pub type DefaultDisplayDriver = linux::LinuxEpdDriver;

#[cfg(feature = "embedded_esp")]
pub type DefaultDisplayDriver = esp::EspEpdDriver;

/// 简化的显示驱动 trait
/// 直接提供 EPD 硬件操作，不包含显示缓冲区
pub trait DisplayDriver {
    /// 初始化显示设备
    fn init(&mut self) -> Result<()>;

    /// 更新并显示帧缓冲区
    fn update_and_display_frame(&mut self, buffer: &[u8]) -> Result<()>;

    /// 进入睡眠模式
    fn sleep(&mut self) -> Result<()>;

    /// 从睡眠模式唤醒
    fn wake(&mut self) -> Result<()>;
}
