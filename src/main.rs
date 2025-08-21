mod bsp;

use epd_waveshare::color::QuadColor;
use epd_waveshare::prelude::WaveshareDisplay;
use log::{debug, info};

use embedded_graphics::{
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

fn main() {
    // 初始化日志
    log::set_max_level(log::LevelFilter::Info);

    // #[cfg(feature = "simulator")]
    env_logger::init();

    info!("墨水屏渲染程序启动");

    let mut board = bsp::Board::new();

    info!("墨水屏即将开始渲染");

    let rect = Rectangle::new(Point::new(10, 10), Size::new(400, 300));

    let _ = rect
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Yellow, 1))
        .draw(&mut board.epd_display);

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
