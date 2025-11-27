// src/driver/display/mod.rs
use crate::common::config::LayoutConfig;
use crate::common::error::{AppError, Result};
use embedded_graphics::geometry::Size;
use embedded_graphics::pixelcolor::BinaryColor;

#[cfg(feature = "simulator")]
mod simulator;

#[cfg(feature = "embedded_linux")]
mod linux_epd_driver;

// 条件编译导入
#[cfg(feature = "simulator")]
pub use simulator::SimulatorEpdDriver as DefaultDisplayDriver;

#[cfg(feature = "embedded_linux")]
pub use linux_epd_driver::LinuxEpdDriver as DefaultDisplayDriver;

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
