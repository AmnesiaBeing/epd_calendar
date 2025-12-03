// src/driver/display/linux.rs
use epd_waveshare::{epd7in5_yrd0750ryf665f60::Epd7in5, prelude::WaveshareDisplay};
use linux_embedded_hal::{Delay, SysfsPin};

use super::DisplayDriver;
use crate::common::error::{AppError, Result};

// 使用条件编译来支持两种 SPI 模式
#[cfg(feature = "spi_bitbang")]
use bitbang_hal::spi_halfduplex::{SPIDevice, SpiConfig};

#[cfg(not(feature = "spi_bitbang"))]
use linux_embedded_hal::SpidevDevice;

// SPI 类型别名
#[cfg(feature = "spi_bitbang")]
type SpiType = SPIDevice<SysfsPin, SysfsPin, SysfsPin, Delay>;

#[cfg(not(feature = "spi_bitbang"))]
type SpiType = SpidevDevice;

pub struct LinuxEpdDriver {
    spi: SpiType,
    epd: Epd7in5<SpiType, SysfsPin, SysfsPin, SysfsPin, Delay>,
}

impl LinuxEpdDriver {
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
    fn init(&mut self) -> Result<()> {
        self.epd
            .wake_up(&mut self.spi, &mut Delay)
            .map_err(|_| AppError::DisplayInit)?;
        Ok(())
    }

    fn sleep(&mut self) -> Result<()> {
        self.epd
            .sleep(&mut self.spi, &mut Delay)
            .map_err(|_| AppError::DisplaySleepFailed)?;
        log::debug!("EPD entered sleep mode");
        Ok(())
    }

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

    fn display_frame(&mut self) -> Result<()> {
        self.epd
            .display_frame(&mut self.spi, &mut Delay)
            .map_err(|e| {
                log::error!("Failed to display frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;
        Ok(())
    }

    fn wake_up(&mut self) -> Result<()> {
        self.init()?;
        log::debug!("EPD woke from sleep");
        Ok(())
    }
}

/// GPIO 初始化辅助函数
fn init_gpio(pin: u64, direction: linux_embedded_hal::sysfs_gpio::Direction) -> Result<SysfsPin> {
    let gpio = SysfsPin::new(pin);
    gpio.export().map_err(|_| AppError::DisplayInit)?;

    // 等待 GPIO 导出完成
    let mut attempts = 0;
    while !gpio.is_exported() {
        embassy_time::Timer::after(embassy_time::Duration::from_millis(10));
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
