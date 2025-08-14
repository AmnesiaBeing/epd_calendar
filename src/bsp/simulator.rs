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
    spi::{Mock as SPIDevice, Transaction},
};
use epd_waveshare::epd7in5_yrd0750ryf665f60::{Display7in5, Epd7in5 as Epd};
use epd_waveshare::prelude::WaveshareDisplay;
use epd_waveshare::{color::*, prelude::*};

use log::info;

pub struct Board {
    pub epd_spi: SPIDevice<u8>,
    pub epd: Epd<SPIDevice<u8>, SysfsPin, SysfsPin, SysfsPin, Delay>,
    pub epd_display: Display7in5,
    pub delay: Delay,
}

impl Board {
    pub fn new() -> Self {
        let epd_busy = SysfsPin::new(&[]);
        let epd_dc = SysfsPin::new(&[]);
        let epd_rst = SysfsPin::new(&[]);
        let mut epd_spi = SPIDevice::new(&[]);

        let epd = Epd::new(
            &mut epd_spi,
            epd_busy,
            epd_dc,
            epd_rst,
            &mut Delay::new(),
            None,
        )
        .expect("eink initalize error");

        let epd_display = Display7in5::default();

        info!("E-Paper display initialized");

        Board {
            epd_spi,
            epd,
            delay: Delay::new(),
            epd_display,
        }
    }
}
