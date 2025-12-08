// src/driver/display/mod.rs

/// 电子墨水屏驱动模块
///
/// 本模块定义了电子墨水屏（EPD）驱动的通用接口和平台特定实现
/// 支持ESP32、Linux和模拟器三种平台的显示驱动
use crate::common::error::Result;

#[cfg(feature = "simulator")]
mod simulator;

#[cfg(feature = "embedded_linux")]
mod linux;

#[cfg(feature = "embedded_esp")]
mod esp;

/// 条件编译导入平台特定驱动
///
/// 根据编译特性选择默认的显示驱动实现
#[cfg(feature = "simulator")]
pub type DefaultDisplayDriver = simulator::SimulatorEpdDriver;

#[cfg(feature = "embedded_linux")]
pub type DefaultDisplayDriver = linux::LinuxEpdDriver;

#[cfg(feature = "embedded_esp")]
pub type DefaultDisplayDriver = esp::EspEpdDriver;

/// 电子墨水屏驱动trait
///
/// 定义电子墨水屏设备的通用操作接口
pub trait DisplayDriver {
    /// 初始化显示设备
    ///
    /// 唤醒EPD显示设备，准备接收数据
    ///
    /// # 返回值
    /// - `Result<()>`: 初始化结果
    fn init(&mut self) -> Result<()>;

    /// 更新帧缓冲区
    ///
    /// 将图像数据写入EPD显示缓冲区
    ///
    /// # 参数
    /// - `buffer`: 图像数据缓冲区
    ///
    /// # 返回值
    /// - `Result<()>`: 更新操作结果
    fn update_frame(&mut self, buffer: &[u8]) -> Result<()>;

    /// 更新部分帧缓冲区
    ///
    /// 更新指定区域的图像数据
    ///
    /// # 参数
    /// - `buffer`: 图像数据缓冲区
    /// - `x`: 区域起始X坐标
    /// - `y`: 区域起始Y坐标
    /// - `width`: 区域宽度
    /// - `height`: 区域高度
    ///
    /// # 返回值
    /// - `Result<()>`: 更新操作结果
    fn update_partial_frame(
        &mut self,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<()>;

    /// 刷新显示缓冲区
    ///
    /// 将缓冲区内容刷新到EPD显示设备
    ///
    /// # 返回值
    /// - `Result<()>`: 刷新操作结果
    fn display_frame(&mut self) -> Result<()>;

    /// 进入休眠模式
    ///
    /// 将EPD设备置于低功耗休眠状态
    ///
    /// # 返回值
    /// - `Result<()>`: 休眠操作结果
    fn sleep(&mut self) -> Result<()>;
}
