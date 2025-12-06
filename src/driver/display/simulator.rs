// src/driver/display/simulator.rs

/// 模拟器电子墨水屏驱动模块
///
/// 本模块实现了模拟器环境下的电子墨水屏（EPD）驱动
/// 使用嵌入式HAL模拟库提供测试和开发环境下的显示功能
use embedded_hal_mock::eh1::{
    delay::NoopDelay as Delay,
    digital::{Mock as SysfsPin, State as PinState, Transaction as PinTransaction},
    spi::Mock as SPIDevice,
};
use epd_waveshare::{epd7in5_yrd0750ryf665f60::Epd7in5, prelude::WaveshareDisplay};
use log::{debug, info};

use super::DisplayDriver;
use crate::common::error::{AppError, Result};

/// SPI 类型别名
///
/// 使用嵌入式HAL模拟库的SPI设备类型
type SpiType = SPIDevice<u8>;

/// 模拟器电子墨水屏驱动结构体
///
/// 封装模拟器环境的EPD驱动功能
pub struct SimulatorEpdDriver {
    /// SPI设备实例
    spi: SpiType,
    /// EPD显示设备实例
    epd: Epd7in5<SpiType, SysfsPin, SysfsPin, SysfsPin, Delay>,
}

impl SimulatorEpdDriver {
    /// 创建新的模拟器EPD驱动实例
    ///
    /// 初始化模拟GPIO引脚和SPI设备
    ///
    /// # 返回值
    /// - `Result<SimulatorEpdDriver>`: 新的EPD驱动实例
    pub async fn new() -> Result<Self> {
        info!("Initializing Simulator EPD driver");

        // 初始化 GPIO 引脚
        let epd_busy = SysfsPin::new(&[PinTransaction::get(PinState::High)]);
        let epd_dc = SysfsPin::new(&[]);
        let epd_rst = SysfsPin::new(&[]);

        // 初始化 SPI
        let mut spi = SPIDevice::new(&[]);

        let epd = Epd7in5::new(&mut spi, epd_busy, epd_dc, epd_rst, &mut Delay, None)
            .map_err(|_| AppError::DisplayInit)?;

        info!("EPD display initialized successfully");
        Ok(Self { spi, epd })
    }
}

impl DisplayDriver for SimulatorEpdDriver {
    /// 初始化显示设备
    ///
    /// 唤醒模拟EPD显示设备，准备接收数据
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
    /// 将模拟EPD设备置于低功耗休眠状态
    ///
    /// # 返回值
    /// - `Result<()>`: 休眠操作结果
    fn sleep(&mut self) -> Result<()> {
        self.epd
            .sleep(&mut self.spi, &mut Delay)
            .map_err(|_| AppError::DisplaySleepFailed)?;
        debug!("EPD entered sleep mode");
        Ok(())
    }

    /// 更新帧缓冲区
    ///
    /// 将图像数据写入模拟EPD显示缓冲区
    ///
    /// # 参数
    /// - `buffer`: 图像数据缓冲区
    ///
    /// # 返回值
    /// - `Result<()>`: 更新操作结果
    fn update_frame(&mut self, buffer: &[u8]) -> Result<()> {
        let mut delay = Delay::new();
        self.epd
            .update_frame(&mut self.spi, buffer, &mut delay)
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
        let mut delay = Delay::new();
        self.epd
            .update_partial_frame(&mut self.spi, &mut delay, buffer, x, y, width, height)
            .map_err(|e| {
                log::error!("Failed to update partial frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;
        Ok(())
    }

    /// 刷新显示缓冲区
    ///
    /// 将缓冲区内容刷新到模拟EPD显示设备
    ///
    /// # 返回值
    /// - `Result<()>`: 刷新操作结果
    fn display_frame(&mut self) -> Result<()> {
        let mut delay = Delay::new();
        self.epd
            .display_frame(&mut self.spi, &mut delay)
            .map_err(|e| {
                log::error!("Failed to display frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;
        Ok(())
    }
}
