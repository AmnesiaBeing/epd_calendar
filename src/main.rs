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

// 在您的main函数中添加这个测试
fn test_draw_direction(display: &mut impl DrawTarget<Color = QuadColor>) -> Result<(), ()> {
    use embedded_graphics::{
        prelude::*,
        primitives::{Line, PrimitiveStyle, Rectangle},
    };

    // 1. 清屏
    let background = Rectangle::new(Point::new(0, 0), Size::new(800, 480))
        .into_styled(PrimitiveStyle::with_fill(QuadColor::White));
    background.draw(display).map_err(|_| ())?;

    // 2. 绘制测试图案 - 检查方向
    // 水平线
    Line::new(Point::new(10, 50), Point::new(100, 50))
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1))
        .draw(display)
        .map_err(|_| ())?;
    // 垂直线
    Line::new(Point::new(10, 60), Point::new(10, 160))
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1))
        .draw(display)
        .map_err(|_| ())?;
    // 对角线
    Line::new(Point::new(20, 70), Point::new(80, 130))
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1))
        .draw(display)
        .map_err(|_| ())?;
    // 矩形
    Rectangle::new(Point::new(110, 50), Size::new(40, 40))
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 2))
        .draw(display)
        .map_err(|_| ())?;

    // 3. 绘制网格 - 检查像素对齐
    for i in 0..10 {
        for j in 0..5 {
            let x = 200 + i * 8;
            let y = 50 + j * 8;
            let pixel = Pixel(Point::new(x, y), QuadColor::Black);
            display.draw_iter(core::iter::once(pixel)).map_err(|_| ())?;
        }
    }

    Ok(())
}

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

    // board.storage.store_kv::<u128>(&0, &1).await;

    // let a = board.storage.fetch_kv::<u128>(&0).await;

    // info!("try read k:0 from kvs, value: {}", a.unwrap());

    // let rect = Rectangle::new(Point::new(10, 10), Size::new(400, 300));

    // let _ = rect
    //     .into_styled(PrimitiveStyle::with_stroke(QuadColor::Yellow, 1))
    //     .draw(&mut board.epd_display);

    // let position = Point::new(10, 100);
    // let style = MonoTextStyle::new(&app::hitokoto_fonts::FULL_WIDTH_FONT, QuadColor::Black);
    // let text = Text::new("你好，世界！", position, style);

    // let _ = text.draw(&mut board.epd_display);

    test_draw_direction(&mut board.epd_display).unwrap();

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
