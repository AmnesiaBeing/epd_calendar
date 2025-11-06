use epd_waveshare::color::QuadColor;
use epd_waveshare::prelude::WaveshareDisplay;
use log::info;

use embassy_executor::Spawner;

use embedded_graphics::{
    mono_font::MonoTextStyle,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Text, TextStyle, TextStyleBuilder},
};

mod app;
mod bsp;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // 初始化日志
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    env_logger::init();
    #[cfg(feature = "embedded_esp")]
    log_to_defmt::setup();

    info!("epd_calendar starting...");

    let mut board = bsp::Board::new();

    info!("epd_calendar running...");

    board.storage.store_kv::<u128>(&0, &1).await;

    let a = board.storage.fetch_kv::<u128>(&0).await;

    info!("try read k:0 from kvs, value: {}", a.unwrap());

    let rect = Rectangle::new(Point::new(10, 10), Size::new(400, 300));

    let _ = rect
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Yellow, 1))
        .draw(&mut board.epd_display);

    let position = Point::new(10, 100);
    let style = MonoTextStyle::new(&app::hitokoto_fonts::FULL_WIDTH_FONT, QuadColor::Black);
    let text = Text::new("你好，世界！", position, style);

    let _ = text.draw(&mut board.epd_display);

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
