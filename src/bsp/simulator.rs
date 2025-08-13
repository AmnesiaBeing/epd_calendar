use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    pixelcolor::Rgb888,
    prelude::*,
    text::{Baseline, Text, TextStyleBuilder},
};
use embedded_hal_mock::eh1::{
    MockError as Error,
    delay::NoopDelay as Delay,
    digital::Mock as SysfsPin,
    spi::{Mock as SpiDevice, Transaction},
};
use epd_waveshare::{color::*, prelude::*};

use epd_waveshare::epd_simulator::EpdSimulator;

pub struct Board {
    pub epd_busy: SysfsPin,
    pub epd_dc: SysfsPin,
    pub epd_rst: SysfsPin,
    pub epd_spi: embedded_hal_mock::common::Generic<embedded_hal_mock::eh1::spi::Transaction<u8>>,
    pub epd: EpdSimulator<QuadColor, (), (), (), (), ()>,
    pub epd_display: Display<800, 480, false, { 800 * 480 * 2 }, QuadColor>,
    pub delay: Delay,
}

impl Board {
    pub fn new() -> Self {
        let epd_busy = SysfsPin::new(&[]);
        let epd_dc = SysfsPin::new(&[]);
        let epd_rst = SysfsPin::new(&[]);
        let mut epd_spi = SpiDevice::new(&[]);
        let delay = Delay::new();
        let epd = EpdSimulator::<QuadColor,(),(),(),(),()>::new(epd_spi, epd_busy, epd_dc, epd_rst, delay, None)
            .unwrap();
        let mut epd_display = Display::<800, 480, false, { 800 * 480 * 2 }, QuadColor>::default();

        Board {
            epd_busy,
            epd_dc,
            epd_rst,
            epd_spi,
            epd,
            delay,
            epd_display,
        }
    }
}
