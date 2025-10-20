use epd_waveshare::color::QuadColor;
use epd_waveshare::prelude::WaveshareDisplay;
use log::{debug, info};

use embassy_executor::Spawner;

use embedded_graphics::{
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

mod bsp;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // 初始化日志
    log::set_max_level(log::LevelFilter::Trace);

    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    env_logger::init();

    #[cfg(feature = "embedded_esp")]
    log_to_defmt::setup();

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

    // spawner.spawn(run().unwrap());
}
