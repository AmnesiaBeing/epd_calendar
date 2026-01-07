/// src/driver/epd/mod.rs
/// 电子墨水屏驱动模块
///
/// 本模块定义了电子墨水屏（EPD）驱动的通用接口和平台特定实现
/// 支持ESP32、Linux和模拟器三种平台的显示驱动
use crate::{common::error::Result, platform::Platform};

#[cfg(feature = "simulator")]
mod simulator;

#[cfg(feature = "tspi")]
mod linux;

#[cfg(feature = "esp32")]
mod esp32;

/// 条件编译导入平台特定驱动
///
/// 根据编译特性选择默认的显示驱动实现
#[cfg(feature = "simulator")]
pub type DefaultDisplayDriver = simulator::SimulatorEpdDriver;

#[cfg(feature = "tspi")]
pub type DefaultDisplayDriver = linux::LinuxEpdDriver;

#[cfg(feature = "esp32")]
pub type DefaultDisplayDriver = esp32::Esp32EpdDriver;

/// 电子墨水屏驱动trait
///
/// 定义电子墨水屏设备的通用操作接口
pub trait DisplayDriver<'p> {
    type P: Platform;

    fn create(peripherals: &'p mut <Self::P as Platform>::Peripherals) -> Self
    where
        Self: Sized;

    /// 更新帧缓冲区
    ///
    /// 将图像数据写入EPD显示缓冲区
    ///
    /// # 参数
    /// - `buffer`: 图像数据缓冲区
    ///
    /// # 返回值
    /// - `Result<()>`: 更新操作结果
    async fn display_frame(&mut self, buffer: &[u8]) -> Result<()>;
}
