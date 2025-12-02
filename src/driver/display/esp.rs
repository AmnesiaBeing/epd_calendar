// src/driver/display/esp.rs

use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::{
    epd7in5_yrd0750ryf665f60::{Display7in5, Epd7in5},
    prelude::WaveshareDisplay,
};
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals::Peripherals,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};

use super::DisplayDriver;
use crate::common::error::{AppError, Result};

// 定义 SPI 设备类型
type EspSpiDevice = ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>;

pub struct EspEpdDriver {
    spi: EspSpiDevice,
    epd: Epd7in5<EspSpiDevice, Input<'static>, Output<'static>, Output<'static>, Delay>,
}

impl EspEpdDriver {
    /// 创建新的 EPD 驱动实例
    ///
    /// 使用固定引脚配置：
    /// - SCK: GPIO6
    /// - SDA/MOSI: GPIO7
    /// - CS: GPIO5
    /// - BUSY: GPIO2
    /// - DC: GPIO3
    /// - RST: GPIO4
    pub fn new(peripherals: Peripherals) -> Result<Self> {
        log::info!("Initializing ESP EPD driver with fixed pin configuration");

        // 配置 SPI 引脚
        let sck = unsafe { peripherals.GPIO6.clone_unchecked() };
        let sda = unsafe { peripherals.GPIO7.clone_unchecked() };
        let cs = Output::new(
            unsafe { peripherals.GPIO5.clone_unchecked() },
            Level::High,
            OutputConfig::default(),
        );

        // 配置 EPD 控制引脚
        let busy = Input::new(
            unsafe { peripherals.GPIO2.clone_unchecked() },
            InputConfig::default().with_pull(Pull::Up),
        );
        let dc = Output::new(
            unsafe { peripherals.GPIO3.clone_unchecked() },
            Level::High,
            OutputConfig::default(),
        );
        let rst = Output::new(
            unsafe { peripherals.GPIO4.clone_unchecked() },
            Level::High,
            OutputConfig::default(),
        );

        // 获取 SPI2 实例
        let spi2 = peripherals.SPI2;

        // 创建 SPI 总线（SpiBus 实现）
        let spi_bus = Spi::new(
            spi2,
            Config::default()
                .with_frequency(Rate::from_khz(100))
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
}

impl DisplayDriver for EspEpdDriver {
    fn init(&mut self) -> Result<()> {
        let mut delay = Delay::new();
        self.epd.wake_up(&mut self.spi, &mut delay).map_err(|e| {
            log::error!("Failed to wake up EPD: {:?}", e);
            AppError::DisplayInit
        })?;
        log::debug!("EPD initialized");
        Ok(())
    }

    fn update_and_display_frame(&mut self, buffer: &[u8]) -> Result<()> {
        let mut delay = Delay::new();
        self.epd
            .update_and_display_frame(&mut self.spi, buffer, &mut delay)
            .map_err(|e| {
                log::error!("Failed to update and display frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;

        log::debug!("EPD frame updated and displayed");
        Ok(())
    }

    fn sleep(&mut self) -> Result<()> {
        let mut delay = Delay::new();
        self.epd.sleep(&mut self.spi, &mut delay).map_err(|e| {
            log::error!("Failed to put EPD to sleep: {:?}", e);
            AppError::DisplaySleepFailed
        })?;
        log::debug!("EPD entered sleep mode");
        Ok(())
    }

    fn wake(&mut self) -> Result<()> {
        self.init()?;
        log::debug!("EPD woke from sleep");
        Ok(())
    }
}
