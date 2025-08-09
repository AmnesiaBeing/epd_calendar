//! 墨水瓶渲染程序主入口

mod bsp;

use embedded_graphics::{mono_font::MonoTextStyleBuilder, prelude::Point, text::{Baseline, Text, TextStyleBuilder}};
use epd_waveshare::{color::QuadColor, graphics::Display};
use log::{debug, info};

fn main() {
    // 初始化日志
    log::set_max_level(log::LevelFilter::Info);

    #[cfg(feature = "simulator")]
    env_logger::init();

    info!("墨水屏渲染程序启动");

    let epd = bsp::simulator::bsp_init().unwrap();

    // Setup the graphics
    let mut display = Display::<800, 480, false, { 800 * 480 * 2 }, QuadColor>::default();

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
    epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)
        .expect("display error");
}
