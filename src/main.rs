mod bsp;

use embedded_graphics::Drawable;
use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    prelude::Point,
    text::{Baseline, Text, TextStyleBuilder},
};
use epd_waveshare::color::QuadColor;
use epd_waveshare::prelude::WaveshareDisplay;
use log::{debug, info};

fn main() {
    // 初始化日志
    log::set_max_level(log::LevelFilter::Info);

    // #[cfg(feature = "simulator")]
    env_logger::init();

    info!("墨水屏渲染程序启动");

    let mut board = bsp::Board::new();

    // Build the style
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
        .text_color(QuadColor::Black)
        .background_color(QuadColor::White)
        .build();
    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    // Draw some text at a certain point using the specified text style
    let _ = Text::with_text_style("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz", Point::new(175, 250), style, text_style)
        .draw(&mut board.epd_display);

    info!("墨水屏即将开始渲染");

    // Show display on e-paper
    board
        .epd
        .update_and_display_frame(
            &mut board.epd_spi,
            board.epd_display.buffer(),
            &mut board.delay,
        )
        .expect("display error");

}
