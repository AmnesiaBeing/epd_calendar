// src/driver/display/tspi.rs
/// 泰山派（Tspi）平台电子墨水屏驱动模块
///
/// 本模块实现了泰山派（Tspi）平台下的电子墨水屏（EPD）驱动
/// 支持两种SPI模式：系统SPI和位操作SPI，适用于泰山派硬件环境
use epd_waveshare::{epd7in5_yrd0750ryf665f60::Epd7in5, prelude::WaveshareDisplay};
use linux_embedded_hal::{Delay, SysfsPin};

use super::DisplayDriver;
use crate::{
    common::{
        GlobalMutex,
        error::{AppError, Result},
    },
    platform::{Platform, tspi::TspiPlatform},
};
use embassy_time::{Duration, Timer};

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

/// 泰山派电子墨水屏驱动结构体
///
/// 封装泰山派（Tspi）平台的EPD驱动功能
pub struct TspiEpdDriver {}

/// GPIO 初始化辅助函数
///
/// 初始化泰山派系统GPIO引脚
///
/// # 参数
/// - `pin`: GPIO引脚编号
/// - `direction`: GPIO方向（输入/输出）
///
/// # 返回值
/// - `Result<SysfsPin>`: 初始化后的GPIO引脚
async fn init_gpio(
    pin: u64,
    direction: linux_embedded_hal::sysfs_gpio::Direction,
) -> Result<SysfsPin> {
    let gpio = SysfsPin::new(pin);
    gpio.export().map_err(|_| AppError::DisplayInit)?;

    // 等待 GPIO 导出完成（异步延时，适配embassy生态）
    let mut attempts = 0;
    while !gpio.is_exported() {
        Timer::after(Duration::from_millis(10)).await;
        attempts += 1;
        if attempts > 100 {
            return Err(AppError::DisplayInit);
        }
    }

    gpio.set_direction(direction)
        .map_err(|_| AppError::DisplayInit)?;

    // 输出引脚默认置高
    if direction == linux_embedded_hal::sysfs_gpio::Direction::Out {
        gpio.set_value(1).map_err(|_| AppError::DisplayInit)?;
    }

    Ok(gpio)
}

impl<'p> DisplayDriver<'p> for TspiEpdDriver {
    // 关联类型指定为泰山派平台
    type P = TspiPlatform;

    /// 更新帧缓冲区（匹配trait定义的异步接口）
    ///
    /// 将图像数据写入EPD显示缓冲区，完成后让屏幕休眠
    ///
    /// # 参数
    /// - `peripherals`: 泰山派平台外设资源（Linux下为逻辑封装，暂未实际使用但保留接口一致性）
    /// - `buffer`: 图像数据缓冲区
    ///
    /// # 返回值
    /// - `Result<()>`: 更新操作结果
    async fn display_frame(
        &mut self,
        peripherals: &'p mut <Self::P as Platform>::Peripherals,
        buffer: &[u8],
    ) -> Result<()> {
        log::info!("Initializing Tspi EPD driver");

        // 1. 初始化泰山派EPD控制引脚（固定编号，可根据硬件实际修改）
        let epd_busy = init_gpio(101, linux_embedded_hal::sysfs_gpio::Direction::In)
            .await
            .map_err(|_| AppError::DisplayInit)?;
        let epd_dc = init_gpio(102, linux_embedded_hal::sysfs_gpio::Direction::Out)
            .await
            .map_err(|_| AppError::DisplayInit)?;
        let epd_rst = init_gpio(97, linux_embedded_hal::sysfs_gpio::Direction::Out)
            .await
            .map_err(|_| AppError::DisplayInit)?;

        // 2. 根据编译特性初始化SPI（系统SPI/位操作SPI）
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

        // 3. 初始化EPD设备
        let mut delay = Delay;
        let mut epd =
            Epd7in5::new(&mut spi, epd_busy, epd_dc, epd_rst, &mut delay, None).map_err(|e| {
                log::error!("Failed to initialize Tspi EPD display: {:?}", e);
                AppError::DisplayInit
            })?;

        // 4. 唤醒设备并更新帧数据
        epd.wake_up(&mut spi, &mut delay).map_err(|e| {
            log::error!("Failed to wake up Tspi EPD display: {:?}", e);
            AppError::DisplayInit
        })?;

        epd.update_frame(&mut spi, buffer, &mut delay)
            .map_err(|e| {
                log::error!("Failed to update Tspi EPD frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;

        // 5. 刷新显示并休眠设备（降低功耗）
        epd.display_frame(&mut spi, &mut delay).map_err(|e| {
            log::error!("Failed to display Tspi EPD frame: {:?}", e);
            AppError::DisplayUpdateFailed
        })?;

        epd.sleep(&mut spi, &mut delay).map_err(|e| {
            log::error!("Failed to sleep Tspi EPD display: {:?}", e);
            AppError::DisplaySleepFailed
        })?;

        log::debug!("Tspi EPD frame updated and displayed successfully");
        Ok(())
    }
}
