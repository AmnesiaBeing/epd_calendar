// src/driver/display/linux.rs

/// Linux平台电子墨水屏驱动模块
///
/// 本模块实现了Linux平台下的电子墨水屏（EPD）驱动
/// 支持两种SPI模式：系统SPI和位操作SPI，适用于不同的硬件环境
use epd_waveshare::{epd7in5_yrd0750ryf665f60::Epd7in5, prelude::WaveshareDisplay};
use linux_embedded_hal::{Delay, SysfsPin};

use super::DisplayDriver;
use crate::common::error::{AppError, Result};

/// 使用条件编译来支持两种 SPI 模式
#[cfg(feature = "spi_bitbang")]
use bitbang_hal::spi_halfduplex::{SPIDevice, SpiConfig};

#[cfg(not(feature = "spi_bitbang"))]
use linux_embedded_hal::SpidevDevice;

/// SPI 类型别名
///
/// 根据编译特性选择不同的SPI实现
#[cfg(feature = "spi_bitbang")]
type SpiType = SPIDevice<SysfsPin, SysfsPin, SysfsPin, Delay>;

#[cfg(not(feature = "spi_bitbang"))]
type SpiType = SpidevDevice;

/// Linux电子墨水屏驱动结构体
///
/// 封装Linux平台的EPD驱动功能
pub struct LinuxEpdDriver {
    /// SPI设备实例
    spi: SpiType,
    /// EPD显示设备实例
    epd: Epd7in5<SpiType, SysfsPin, SysfsPin, SysfsPin, Delay>,
}

impl LinuxEpdDriver {
    /// 创建新的Linux EPD驱动实例
    ///
    /// 根据编译特性选择不同的SPI初始化方式
    ///
    /// # 返回值
    /// - `Result<LinuxEpdDriver>`: 新的EPD驱动实例
    pub async fn new() -> Result<Self> {
        log::info!("Initializing Linux EPD driver");

        // 初始化 GPIO 引脚
        let epd_busy = init_gpio(101, linux_embedded_hal::sysfs_gpio::Direction::In)
            .map_err(|_| AppError::DisplayInit)?;
        let epd_dc = init_gpio(102, linux_embedded_hal::sysfs_gpio::Direction::Out)
            .map_err(|_| AppError::DisplayInit)?;
        let epd_rst = init_gpio(97, linux_embedded_hal::sysfs_gpio::Direction::Out)
            .map_err(|_| AppError::DisplayInit)?;

        // 根据特性选择 SPI 初始化方式
        #[cfg(feature = "spi_bitbang")]
        let mut spi = {
            let mosi = init_gpio(147, linux_embedded_hal::sysfs_gpio::Direction::Out)
                .await
                .map_err(|_| AppError::DisplayInit)?;
            let sck = init_gpio(146, linux_embedded_hal::sysfs_gpio::Direction::Out)
                .await
                .map_err(|_| AppError::DisplayInit)?;
            let cs = init_gpio(150, linux_embedded_hal::sysfs_gpio::Direction::Out)
                .await
                .map_err(|_| AppError::DisplayInit)?;

            let config = SpiConfig::default();
            SPIDevice::new(embedded_hal::spi::MODE_0, mosi, sck, cs, Delay, config)
        };

        #[cfg(not(feature = "spi_bitbang"))]
        let mut spi = SpidevDevice::open("/dev/spidev3.0").map_err(|_| AppError::DisplayInit)?;

        let epd = Epd7in5::new(&mut spi, epd_busy, epd_dc, epd_rst, &mut Delay, None)
            .map_err(|_| AppError::DisplayInit)?;

        log::info!("EPD display initialized successfully");
        Ok(Self { spi, epd })
    }
}

impl DisplayDriver for LinuxEpdDriver {
    /// 初始化显示设备
    ///
    /// 唤醒EPD显示设备，准备接收数据
    ///
    /// # 返回值
    /// - `Result<()>`: 初始化结果
    fn init(&mut self) -> Result<()> {
        self.epd
            .wake_up(&mut self.spi, &mut Delay)
            .map_err(|_| AppError::DisplayInit)?;
        Ok(())
    }

    /// 进入休眠模式
    ///
    /// 将EPD设备置于低功耗休眠状态
    ///
    /// # 返回值
    /// - `Result<()>`: 休眠操作结果
    fn sleep(&mut self) -> Result<()> {
        self.epd
            .sleep(&mut self.spi, &mut Delay)
            .map_err(|_| AppError::DisplaySleepFailed)?;
        log::debug!("EPD entered sleep mode");
        Ok(())
    }

    /// 更新帧缓冲区
    ///
    /// 将图像数据写入EPD显示缓冲区
    ///
    /// # 参数
    /// - `buffer`: 图像数据缓冲区
    ///
    /// # 返回值
    /// - `Result<()>`: 更新操作结果
    fn update_frame(&mut self, buffer: &[u8]) -> Result<()> {
        self.epd
            .update_frame(&mut self.spi, buffer, &mut Delay)
            .map_err(|e| {
                log::error!("Failed to update frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;

        log::debug!("EPD frame updated and displayed");
        Ok(())
    }

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
    ) -> Result<()> {
        self.epd
            .update_partial_frame(&mut self.spi, &mut Delay, buffer, x, y, width, height)
            .map_err(|e| {
                log::error!("Failed to update partial frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;
        Ok(())
    }

    /// 刷新显示缓冲区
    ///
    /// 将缓冲区内容刷新到EPD显示设备
    ///
    /// # 返回值
    /// - `Result<()>`: 刷新操作结果
    fn display_frame(&mut self) -> Result<()> {
        self.epd
            .display_frame(&mut self.spi, &mut Delay)
            .map_err(|e| {
                log::error!("Failed to display frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;
        Ok(())
    }
}

/// GPIO 初始化辅助函数
///
/// 初始化Linux系统GPIO引脚
///
/// # 参数
/// - `pin`: GPIO引脚编号
/// - `direction`: GPIO方向（输入/输出）
///
/// # 返回值
/// - `Result<SysfsPin>`: 初始化后的GPIO引脚
fn init_gpio(pin: u64, direction: linux_embedded_hal::sysfs_gpio::Direction) -> Result<SysfsPin> {
    let gpio = SysfsPin::new(pin);
    gpio.export().map_err(|_| AppError::DisplayInit)?;

    // 等待 GPIO 导出完成
    let mut attempts = 0;
    while !gpio.is_exported() {
        let _ = embassy_time::Timer::after(embassy_time::Duration::from_millis(10));
        attempts += 1;
        if attempts > 100 {
            return Err(AppError::DisplayInit);
        }
    }

    gpio.set_direction(direction)
        .map_err(|_| AppError::DisplayInit)?;

    if direction == linux_embedded_hal::sysfs_gpio::Direction::Out {
        gpio.set_value(1).map_err(|_| AppError::DisplayInit)?;
    }

    Ok(gpio)
}
