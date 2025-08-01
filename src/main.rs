//! 墨水瓶渲染程序主入口

use log::{debug, info};

#[cfg(feature = "embedded_linux")]
use bitbang_hal::spi_halfduplex::{SPI as SpiDevice, SpiConfig};
use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    prelude::*,
    text::{Baseline, Text, TextStyleBuilder},
};
#[cfg(feature = "simulator")]
use embedded_hal_mock::eh1::{
    MockError as Error,
    delay::NoopDelay as Delay,
    digital::Mock as SysfsPin,
    spi::{Mock as SpiDevice, Transaction},
};
use epd_waveshare::{
    color::*,
    epd7in5_yrd0750ryf665f60::{Display7in5, Epd7in5},
    prelude::*,
};
#[cfg(feature = "embedded_linux")]
use linux_embedded_hal::{Delay, SPIError as Error, SysfsPin, sysfs_gpio::Direction};

fn main() -> Result<(), Error> {
    // 初始化日志
    log::set_max_level(log::LevelFilter::Info);

    #[cfg(feature = "simulator")]
    env_logger::init();

    info!("墨水屏渲染程序启动");

    let mut delay = Delay {};

    let mut spi;
    #[cfg(feature = "embedded_linux")]
    {
        let mosi = SysfsPin::new(147);
        mosi.export().expect("miso export");
        while !mosi.is_exported() {}
        mosi.set_direction(Direction::In).expect("CS Direction");
        mosi.set_value(1).expect("CS Value set to 1");

        let sck = SysfsPin::new(146);
        sck.export().expect("miso export");
        while !sck.is_exported() {}
        sck.set_direction(Direction::In).expect("CS Direction");
        sck.set_value(1).expect("CS Value set to 1");

        let config = SpiConfig::default();

        spi = SpiDevice::new(embedded_hal::spi::MODE_0, mosi, sck, delay, config);
    }
    #[cfg(feature = "simulator")]
    {
        spi = SpiDevice::new(&[]);
    }

    // Configure Digital I/O Pin to be used as Chip Select for SPI
    let cs;
    #[cfg(feature = "embedded_linux")]
    {
        cs = SysfsPin::new(150);
        cs.export().expect("cs export");
        while !cs.is_exported() {}
        cs.set_direction(Direction::Out).expect("CS Direction");
        cs.set_value(1).expect("CS Value set to 1");
    }
    #[cfg(feature = "simulator")]
    {
        cs = SysfsPin::new(&[]);
    }

    let busy;
    #[cfg(feature = "embedded_linux")]
    {
        busy = SysfsPin::new(101);
        busy.export().expect("busy export");
        while !busy.is_exported() {}
        busy.set_direction(Direction::In).expect("busy Direction");
        //busy.set_value(1).expect("busy Value set to 1");
    }
    #[cfg(feature = "simulator")]
    {
        busy = SysfsPin::new(&[]);
    }

    let dc;
    #[cfg(feature = "embedded_linux")]
    {
        dc = SysfsPin::new(102);
        dc.export().expect("dc export");
        while !dc.is_exported() {}
        dc.set_direction(Direction::Out).expect("dc Direction");
        dc.set_value(1).expect("dc Value set to 1");
    }
    #[cfg(feature = "simulator")]
    {
        dc = SysfsPin::new(&[]);
    }

    let rst;
    #[cfg(feature = "embedded_linux")]
    {
        rst = SysfsPin::new(97);
        rst.export().expect("rst export");
        while !rst.is_exported() {}
        rst.set_direction(Direction::Out).expect("rst Direction");
        rst.set_value(1).expect("rst Value set to 1");
    }
    #[cfg(feature = "simulator")]
    {
        rst = SysfsPin::new(&[]);
    }

    info!("SPI, CS, DC, RST, and BUSY pins initialized");

    // Setup the epd
    let mut epd7in5 =
        Epd7in5::new(&mut spi, busy, dc, rst, &mut delay, None).expect("eink initalize error");

    // Setup the graphics
    let mut display = Display7in5::default();

    // Build the style
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(QuadColor::White)
        .background_color(QuadColor::Black)
        .build();
    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    // Draw some text at a certain point using the specified text style
    let _ = Text::with_text_style("It's working-WoB!", Point::new(175, 250), style, text_style)
        .draw(&mut display);

    // Show display on e-paper
    epd7in5
        .update_and_display_frame(&mut spi, display.buffer(), &mut delay)
        .expect("display error");

    // Going to sleep
    let _ = epd7in5.sleep(&mut spi, &mut delay);

    Ok(())
}
