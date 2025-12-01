// src/driver/display/esp.rs

use embedded_hal::{
    digital::{InputPin, OutputPin},
    spi::{SpiBus, SpiDevice},
};
use epd_waveshare::{epd7in5_yrd0750ryf665f60::Epd7in5, prelude::WaveshareDisplay};
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, Output},
    peripherals::SPI2,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};

use super::DisplayDriver;
use crate::common::error::{AppError, Result};

pub struct EspEpdDriver {
    spi: Spi<'static, Blocking>,
    epd: Epd7in5<Spi<'static, Blocking>, Input<'static>, Output<'static>, Output<'static>, Delay>,
}

impl EspEpdDriver {
    pub fn new(
        sck: impl Into<Output<'static>>,
        sda: impl Into<Output<'static>>,
        cs: impl Into<Output<'static>>,
        busy: impl Into<Input<'static>>,
        dc: impl Into<Output<'static>>,
        rst: impl Into<Output<'static>>,
        spi: SPI2,
    ) -> Result<Self> {
        log::info!("Initializing ESP EPD driver");

        let delay = Delay::new();

        // 创建SPI实例
        let mut spi = Spi::new(
            spi,
            Config::default()
                .with_frequency(Rate::from_khz(100))
                .with_mode(Mode::_0),
        )
        .map_err(|e| {
            log::error!("Failed to initialize SPI: {:?}", e);
            AppError::DisplayInit
        })?
        .with_cs(cs.into())
        .with_sck(sck.into())
        .with_sio0(sda.into());

        // 创建EPD实例
        let mut epd = Epd7in5::new(
            &mut spi,
            busy.into(),
            dc.into(),
            rst.into(),
            &mut delay,
            None,
        )
        .map_err(|e| {
            log::error!("Failed to initialize EPD: {:?}", e);
            AppError::DisplayInit
        })?;

        log::info!("EPD display initialized successfully");
        Ok(Self { spi, epd })
    }
}

impl DisplayDriver for EspEpdDriver {
    fn init(&mut self) -> Result<()> {
        self.epd.wake_up(&mut self.spi).map_err(|e| {
            log::error!("Failed to wake up EPD: {:?}", e);
            AppError::DisplayInit
        })?;
        log::debug!("EPD initialized");
        Ok(())
    }

    fn update_and_display_frame(&mut self, buffer: &[u8]) -> Result<()> {
        // 直接使用 EPD 的方法更新和显示帧
        self.epd
            .update_and_display_frame(buffer, &mut self.spi)
            .map_err(|e| {
                log::error!("Failed to update and display frame: {:?}", e);
                AppError::DisplayUpdateFailed
            })?;

        log::debug!("EPD frame updated and displayed");
        Ok(())
    }

    fn sleep(&mut self) -> Result<()> {
        self.epd.sleep(&mut self.spi).map_err(|e| {
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
