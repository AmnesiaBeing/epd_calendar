// src/driver/display/simulator.rs
use embedded_hal_mock::eh1::{
    delay::NoopDelay as Delay,
    digital::{Mock as SysfsPin, State as PinState, Transaction as PinTransaction},
    spi::Mock as SPIDevice,
};
use epd_waveshare::{epd7in5_yrd0750ryf665f60::Epd7in5, prelude::WaveshareDisplay};
use log::{debug, info};

use super::DisplayDriver;
use crate::common::error::{AppError, Result};

// SPI 类型别名
type SpiType = SPIDevice<u8>;

pub struct SimulatorEpdDriver {
    spi: SpiType,
    epd: Epd7in5<SpiType, SysfsPin, SysfsPin, SysfsPin, Delay>,
}

impl SimulatorEpdDriver {
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
    fn init(&mut self) -> Result<()> {
        self.epd
            .wake_up(&mut self.spi, &mut Delay)
            .map_err(|_| AppError::DisplayInit)?;
        Ok(())
    }

    fn update_and_display_frame(&mut self, buffer: &[u8]) -> Result<()> {
        // 直接使用 EPD 的方法更新和显示帧
        self.epd
            .update_and_display_frame(&mut self.spi, buffer, &mut Delay)
            .map_err(|_| AppError::DisplayUpdateFailed)?;

        debug!("EPD frame updated and displayed");
        Ok(())
    }

    fn sleep(&mut self) -> Result<()> {
        self.epd
            .sleep(&mut self.spi, &mut Delay)
            .map_err(|_| AppError::DisplaySleepFailed)?;
        debug!("EPD entered sleep mode");
        Ok(())
    }

    fn wake(&mut self) -> Result<()> {
        self.init()?;
        debug!("EPD woke from sleep");
        Ok(())
    }
}
