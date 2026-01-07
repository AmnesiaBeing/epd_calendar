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
use log::{debug, error, info};

use super::DisplayDriver;
use crate::{
    common::error::{AppError, Result},
    platform::{Platform, simulator::SimulatorPlatform},
};

/// 模拟器电子墨水屏驱动结构体
///
/// 封装模拟器环境的EPD驱动功能（简化结构体，仅保留标识作用）
pub struct SimulatorEpdDriver {}

impl<'p> DisplayDriver<'p> for SimulatorEpdDriver {
    // 关联类型指定为模拟器平台
    type P = SimulatorPlatform;

    /// 更新帧缓冲区（匹配统一的异步接口）
    ///
    /// 将图像数据写入模拟EPD显示缓冲区，完成模拟显示流程
    ///
    /// # 参数
    /// - `peripherals`: 模拟器平台外设资源（仅保留接口一致性，无实际作用）
    /// - `buffer`: 图像数据缓冲区
    ///
    /// # 返回值
    /// - `Result<()>`: 更新操作结果
    async fn display_frame(
        &mut self,
        peripherals: &'p mut <Self::P as Platform>::Peripherals,
        buffer: &[u8],
    ) -> Result<()> {
        info!("Initializing Simulator EPD driver");

        // 1. 初始化模拟GPIO引脚（保持原有模拟逻辑）
        let epd_busy = SysfsPin::new(&[PinTransaction::get(PinState::High)]);
        let epd_dc = SysfsPin::new(&[]);
        let epd_rst = SysfsPin::new(&[]);

        // 2. 初始化模拟SPI设备
        let mut spi = SPIDevice::new(&[]);
        let mut delay = Delay::new();

        // 3. 初始化模拟EPD设备
        let mut epd =
            Epd7in5::new(&mut spi, epd_busy, epd_dc, epd_rst, &mut delay, None).map_err(|e| {
                error!("Failed to initialize Simulator EPD display: {:?}", e);
                AppError::DisplayInit
            })?;

        info!("Simulator EPD display initialized successfully");

        // 4. 模拟唤醒设备流程
        epd.wake_up(&mut spi, &mut delay).map_err(|e| {
            error!("Failed to wake up Simulator EPD display: {:?}", e);
            AppError::DisplayInit
        })?;

        // 5. 模拟更新帧数据
        epd.update_frame(&mut spi, buffer, &mut delay)
            .map_err(|e| {
                error!("Failed to update Simulator EPD frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;

        // 6. 模拟刷新显示
        epd.display_frame(&mut spi, &mut delay).map_err(|e| {
            error!("Failed to display Simulator EPD frame: {:?}", e);
            AppError::DisplayUpdateFailed
        })?;

        // 7. 模拟休眠设备
        epd.sleep(&mut spi, &mut delay).map_err(|e| {
            error!("Failed to sleep Simulator EPD display: {:?}", e);
            AppError::DisplaySleepFailed
        })?;

        debug!("Simulator EPD frame updated and displayed successfully");
        Ok(())
    }
}
