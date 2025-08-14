use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    pixelcolor::Rgb888,
    prelude::*,
    text::{Baseline, Text, TextStyleBuilder},
};
use embedded_hal_mock::eh1::{
    MockError as Error,
    delay::NoopDelay as Delay,
    digital::{Mock as SysfsPin, State as PinState, Transaction as PinTransaction},
    spi::{Mock as SPIDevice, Transaction},
};
use epd_waveshare::epd_simulator::{Display, EpdSimulator as Epd};
use epd_waveshare::prelude::WaveshareDisplay;
use epd_waveshare::{color::*, prelude::*};

use log::info;

// 在结构体外部定义静态缓冲区
static mut BUFFER: [u8; 800 * 480] = [0x00; 800 * 480];

pub struct Board {
    pub epd_spi: SPIDevice<u8>,
    pub epd: Epd<QuadColor, SPIDevice<u8>, SysfsPin, SysfsPin, SysfsPin, Delay>,
    pub epd_display: Display<'static, QuadColor>, // 注意生命周期改为 'static
    pub delay: Delay,
}

impl Board {
    pub fn new() -> Self {
        let epd_busy = SysfsPin::new(&[PinTransaction::get(PinState::High)]);
        let epd_dc = SysfsPin::new(&[]);
        let epd_rst = SysfsPin::new(&[]);
        let mut epd_spi = SPIDevice::new(&[]);

        let epd = Epd::new_with_size(
            &mut epd_spi,
            epd_busy,
            epd_dc,
            epd_rst,
            &mut Delay::new(),
            None,
            800,
            480,
        )
        .expect("eink initalize error");

        // 借用结构体内部的缓冲区
        let epd_display = unsafe { Display::new(800, 480, &mut BUFFER[..], false).unwrap() };

        info!("E-Paper display initialized");

        Board {
            epd_spi,
            epd,
            delay: Delay::new(),
            epd_display,
        }
    }
}
