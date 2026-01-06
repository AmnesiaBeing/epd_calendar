// src/driver/display/esp.rs

/// ESP32平台电子墨水屏驱动模块
///
/// 本模块实现了ESP32平台下的电子墨水屏（EPD）驱动
/// 使用Waveshare EPD库和ESP32硬件SPI接口控制7.5英寸电子墨水屏
use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::{epd7in5_yrd0750ryf665f60::Epd7in5, prelude::WaveshareDisplay};
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig},
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};

use super::DisplayDriver;
use crate::{
    common::error::{AppError, Result},
    platform::{Platform, esp32::Esp32Platform},
};

/// ESP32 SPI设备类型别名
///
/// 使用ExclusiveDevice包装SPI总线，提供独占访问
/// 确保SPI通信的原子性和可靠性
type Esp32SpiDevice<'a> = ExclusiveDevice<Spi<'a, Blocking>, Output<'a>, Delay>;

/// ESP32电子墨水屏驱动结构体
///
/// 封装ESP32平台的EPD驱动功能
pub struct Esp32EpdDriver<'a> {
    /// SPI设备实例
    spi: Esp32SpiDevice<'a>,
    /// EPD显示设备实例
    epd: Epd7in5<Esp32SpiDevice<'a>, Input<'a>, Output<'a>, Output<'a>, Delay>,
}

impl<'a> DisplayDriver<'a> for Esp32EpdDriver<'a> {
    type P = Esp32Platform;
    /// 创建新的EPD驱动实例
    ///
    /// 使用固定引脚配置：
    /// - SCK: GPIO22
    /// - SDA/MOSI: GPIO23
    /// - CS: GPIO21
    /// - BUSY: GPIO18
    /// - DC: GPIO20
    /// - RST: GPIO19
    ///
    /// # 参数
    /// - `peripherals`: ESP32外设实例
    ///
    /// # 返回值
    /// - `Result<Esp32EpdDriver>`: 新的EPD驱动实例
    fn new(peripherals: &'a mut <Self::P as Platform>::Peripherals) -> Result<Self> {
        log::info!("Initializing ESP EPD driver with fixed pin configuration");

        // 配置 SPI 引脚
        let sck = peripherals.GPIO22.reborrow();
        let sda = peripherals.GPIO23.reborrow();
        let cs = Output::new(
            peripherals.GPIO21.reborrow(),
            Level::High,
            OutputConfig::default(),
        );

        // 配置 EPD 控制引脚
        let busy = Input::new(
            peripherals.GPIO18.reborrow(),
            InputConfig::default(),
        );
        let dc = Output::new(
            peripherals.GPIO20.reborrow(),
            Level::High,
            OutputConfig::default(),
        );
        let rst = Output::new(
            peripherals.GPIO19.reborrow(),
            Level::High,
            OutputConfig::default(),
        );

        // 获取 SPI2 实例
        let spi2 = peripherals.SPI2.reborrow();

        // 创建 SPI 总线
        let spi_bus = Spi::new(
            spi2,
            Config::default()
                .with_frequency(Rate::from_mhz(10))
                .with_mode(Mode::_0),
        )
        .map_err(|e| {
            log::error!("Failed to initialize SPI bus: {:?}", e);
            AppError::DisplayInit
        })?
        .with_sck(sck)
        .with_sio0(sda);

        // 创建 Delay 用于 ExclusiveDevice
        let device_delay = Delay::new();

        // 创建 ExclusiveDevice
        let mut spi_device =
            ExclusiveDevice::new(spi_bus, cs, device_delay).map_err(|_| AppError::DisplayInit)?;

        // 创建 Delay 用于 EPD 初始化
        let mut epd_delay = Delay::new();

        // 创建 EPD 实例
        let epd = Epd7in5::new(
            &mut spi_device, // 这里需要可变引用
            busy,
            dc,
            rst,
            &mut epd_delay,
            None,
        )
        .map_err(|e| {
            log::error!("Failed to initialize EPD: {:?}", e);
            AppError::DisplayInit
        })?;

        log::info!("EPD display initialized successfully");
        Ok(Self {
            spi: spi_device,
            epd,
        })
    }

    /// 进入休眠模式
    ///
    /// 将EPD设备置于低功耗休眠状态
    ///
    /// # 返回值
    /// - `Result<()>`: 休眠操作结果
    fn sleep(&mut self) -> Result<()> {
        let mut delay = Delay::new();
        self.epd.sleep(&mut self.spi, &mut delay).map_err(|e| {
            log::error!("Failed to put EPD to sleep: {:?}", e);
            AppError::DisplaySleepFailed
        })?;
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
    /// 将缓冲区内容刷新到EPD显示设备
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
