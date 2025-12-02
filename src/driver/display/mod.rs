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

pub trait DisplayDriver {
    /// 初始化显示设备
    fn init(&mut self) -> Result<()>;

    /// 更新缓冲区
    fn update_frame(&mut self, buffer: &[u8]) -> Result<()>;

    /// 更新部分帧缓冲区
    fn update_partial_frame(
        &mut self,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<()>;

    /// 刷新显示缓冲区
    fn display_frame(&mut self) -> Result<()>;

    /// 进入睡眠模式
    fn sleep(&mut self) -> Result<()>;

    /// 从睡眠模式唤醒
    fn wake_up(&mut self) -> Result<()>;
}
