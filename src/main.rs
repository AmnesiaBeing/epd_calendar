use epd_waveshare::color::QuadColor;
use epd_waveshare::prelude::WaveshareDisplay;
use log::info;

use embassy_executor::Spawner;

use embedded_graphics::{
    image::Image,
    mono_font::MonoTextStyle,
    prelude::*,
    primitives::{CornerRadii, Line, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

use crate::drv::{
    battery_renderer::render_battery_status,
    datetime_renderer::{DateConfig, Meridiem, TimeConfig, render_datetime},
    network_renderer::render_network_status,
};

mod app;
mod bsp;
mod drv;

// 把显示元素的位置固定下来（左上角）
const SEPARATOR_LINE_X_1: i32 = 10;
const SEPARATOR_LINE_X_2: i32 = 790;
const SEPARATOR_LINE_Y_1: i32 = 180;
const SEPARATOR_LINE_Y_2: i32 = 400;
const SEPARATOR_LINE_3_X: i32 = 8 * 24 + 20;
const SEPARATOR_LINE_3_Y_1: i32 = SEPARATOR_LINE_Y_1 + 10;
const SEPARATOR_LINE_3_Y_2: i32 = SEPARATOR_LINE_Y_2 - 10;
const TIME_ELEMENT_WIDTH: i32 = 48; // 每个时间都是等宽字体，宽度48px，高度128px
const TIME_ELEMENT_HEIGHT: i32 = 128;
const TIME_POSITION: Point = Point::new(
    400 - 2 * TIME_ELEMENT_WIDTH - TIME_ELEMENT_WIDTH / 2,
    (SEPARATOR_LINE_Y_1 - TIME_ELEMENT_HEIGHT) / 2,
);
const ICON_WIDTH: i32 = 64;
const ICON_HEIGHT: i32 = 64;
const BATTERY_POSITION: Point = Point::new(800 - 10 - ICON_WIDTH, 10);
const BOLT_POSITION: Point = Point::new(800 - 10 - 5 - 2 * ICON_WIDTH, 10);

pub struct InkDisplay {
    pub time: TimeConfig,
    pub date: DateConfig,
    pub weekday: String,
    pub temperature: i32,
    pub humidity: u8,
    pub weather_condition: WeatherCondition,
    pub battery_level: u8,
    pub wifi_connected: bool,
    pub quote: String,
    pub quote_author: String,
}

#[derive(Clone, Copy)]
pub enum WeatherCondition {
    Sunny,
    Cloudy,
    Rainy,
    Snowy,
    Foggy,
}

impl Default for InkDisplay {
    fn default() -> Self {
        Self {
            time: TimeConfig {
                hour_tens: 8,
                hour_ones: 8,
                minute_tens: 8,
                minute_ones: 8,
                show_meridiem: true,
                meridiem: Meridiem::AM,
            },
            date: DateConfig {
                year: 2025,
                month: 11,
                day: 26,
                weekday: 2,
            },
            weekday: "星期一".to_string(),
            temperature: 20,
            humidity: 50,
            weather_condition: WeatherCondition::Sunny,
            battery_level: 100,
            wifi_connected: true,
            quote: "今日格言将显示在这里".to_string(),
            quote_author: "佚名".to_string(),
        }
    }
}

impl InkDisplay {
    pub fn draw<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 1. 绘制背景&分割线
        self.clear_screen(display);

        // 2. 绘制顶部状态信息，包括网络、电量、充电状态
        self.draw_status_bar(display);

        // 3. 绘制时间
        self.draw_time_section(display);

        // 4. 绘制农历
        // self.draw_time_section(display);

        // 5. 绘制天气信息
        // self.draw_weather_section(display);

        // 6. 绘制格言区域
        self.draw_quote_section(display);
    }

    fn clear_screen<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let _ = Rectangle::new(Point::new(0, 0), Size::new(800, 480))
            .into_styled(PrimitiveStyle::with_fill(QuadColor::White))
            .draw(display);

        let _ = Line::new(
            Point {
                x: SEPARATOR_LINE_X_1,
                y: SEPARATOR_LINE_Y_1,
            },
            Point {
                x: SEPARATOR_LINE_X_2,
                y: SEPARATOR_LINE_Y_1,
            },
        )
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1))
        .draw(display);

        let _ = Line::new(
            Point {
                x: SEPARATOR_LINE_X_1,
                y: SEPARATOR_LINE_Y_2,
            },
            Point {
                x: SEPARATOR_LINE_X_2,
                y: SEPARATOR_LINE_Y_2,
            },
        )
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1))
        .draw(display);

        let _ = Line::new(
            Point {
                x: SEPARATOR_LINE_3_X,
                y: SEPARATOR_LINE_3_Y_1,
            },
            Point {
                x: SEPARATOR_LINE_3_X,
                y: SEPARATOR_LINE_3_Y_2,
            },
        )
        .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1))
        .draw(display);
    }

    fn draw_status_bar<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let battery_status = drv::battery_renderer::BatteryStatus {
            level: self.battery_level,
            is_charging: true,
        };
        let _ = render_battery_status(display, &battery_status);

        let network_status = drv::network_renderer::NetworkStatus { is_connected: true };
        let _ = render_network_status(display, &network_status);
    }

    fn draw_time_section<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let _ = render_datetime(display, &self.date, &self.time);
    }

    fn draw_weather_section<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
    }

    fn draw_quote_section<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        drv::hitokoto_renderer::render_next_hitokoto(display);
    }
}

// 在主函数中使用
pub fn render_display<D>(display: &mut D)
where
    D: DrawTarget<Color = QuadColor>,
{
    let ink_display = InkDisplay::default();
    ink_display.draw(display);
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

    render_display(&mut board.epd_display);

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
