use embedded_hal::delay;
use log::{debug, info};

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

pub fn bsp_init() -> Result<
    EpdSimulator<
        QuadColor,
        embedded_hal_mock::common::Generic<Transaction<u8>>,
        embedded_hal_mock::common::Generic<embedded_hal_mock::eh1::digital::Transaction>,
        embedded_hal_mock::common::Generic<embedded_hal_mock::eh1::digital::Transaction>,
        embedded_hal_mock::common::Generic<embedded_hal_mock::eh1::digital::Transaction>,
        Delay,
    >,
    Error,
> {
    let mut spi = SpiDevice::new(&[]);
    let mut busy = SysfsPin::new(&[]);
    let mut dc = SysfsPin::new(&[]);
    let mut rst = SysfsPin::new(&[]);
    let mut delay = Delay::new();

    // Setup the epd
    let mut epd: EpdSimulator<QuadColor, embedded_hal_mock::common::Generic<Transaction<u8>>, embedded_hal_mock::common::Generic<embedded_hal_mock::eh1::digital::Transaction>, embedded_hal_mock::common::Generic<embedded_hal_mock::eh1::digital::Transaction>, embedded_hal_mock::common::Generic<embedded_hal_mock::eh1::digital::Transaction>, Delay> = EpdSimulator::<QuadColor, SpiDevice<u8>,SysfsPin,SysfsPin,SysfsPin,Delay>::new_with_size(
        &mut spi, busy, dc, rst, &mut delay, None, 800, 480,
    )
    .expect("eink initalize error");

    Ok(epd)
}
