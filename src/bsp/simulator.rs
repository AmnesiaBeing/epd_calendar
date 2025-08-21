use embedded_hal_mock::eh1::{
    delay::NoopDelay as Delay,
    digital::{Mock as SysfsPin, State as PinState, Transaction as PinTransaction},
    spi::Mock as SPIDevice,
};
use epd_waveshare::prelude::WaveshareDisplay;
use epd_waveshare::{color::*, prelude::*};
use epd_waveshare::{epd_simulator::EpdSimulator, graphics::VarDisplay};

use log::info;

// 在结构体外部定义静态缓冲区
static mut BUFFER: [u8; 800 * 480] = [0x00; 800 * 480];

pub struct Board {
    pub epd_spi: SPIDevice<u8>,
    pub epd: EpdSimulator<QuadColor, SPIDevice<u8>, SysfsPin, SysfsPin, SysfsPin, Delay>,
    pub epd_display: VarDisplay<'static, QuadColor>,
    pub delay: Delay,
}

impl Board {
    pub fn new() -> Self {
        let epd_busy = SysfsPin::new(&[PinTransaction::get(PinState::High)]);
        let epd_dc = SysfsPin::new(&[]);
        let epd_rst = SysfsPin::new(&[]);
        let mut epd_spi = SPIDevice::new(&[]);

        let epd = EpdSimulator::new_with_size(
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
        let epd_display = unsafe { VarDisplay::new(800, 480, &mut BUFFER[..], false).unwrap() };

        info!("E-Paper display initialized");

        Board {
            epd_spi,
            epd,
            delay: Delay::new(),
            epd_display,
        }
    }
}
