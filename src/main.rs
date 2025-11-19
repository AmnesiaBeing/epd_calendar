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

use crate::drv::battery_renderer::render_battery_status;

mod app;
mod bsp;
mod drv;

// 把显示元素的位置固定下来（左上角）
const SEPARATOR_LINE_X_1: i32 = 10;
const SEPARATOR_LINE_X_2: i32 = 790;
const SEPARATOR_LINE_Y_1: i32 = 140;
const SEPARATOR_LINE_Y_2: i32 = 360;
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

pub fn draw_weather_icon<D>(
    icon: drv::weather_icons::WeatherIcon,
    position: Point,
    display: &mut D,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let image_data = drv::weather_icons::get_icon_data(icon);
    let width = drv::weather_icons::WEATHER_ICON_SIZE;
    let height = drv::weather_icons::WEATHER_ICON_SIZE;

    // 计算每行字节数
    let bytes_per_row = (width + 7) / 8;

    // 绘制每个像素
    let pixels = (0..height).flat_map(move |y| {
        (0..width).map(move |x| {
            let byte_index = (y * bytes_per_row + x / 8) as usize;
            let bit_index = 7 - (x % 8);

            let color = if byte_index < image_data.len() {
                let byte = image_data[byte_index];
                let bit = (byte >> bit_index) & 1;
                if bit == 1 {
                    QuadColor::Black
                } else {
                    QuadColor::White
                }
            } else {
                QuadColor::White
            };

            Pixel(Point::new(x as i32, y as i32) + position, color)
        })
    });

    display.draw_iter(pixels)
}

pub struct InkDisplay {
    pub time: String,
    pub date: String,
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
            time: "12:00".to_string(),
            date: "2024-01-01".to_string(),
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
        // self.draw_time_section(display);

        // 4. 绘制农历
        // self.draw_time_section(display);

        // 5. 绘制天气信息
        self.draw_weather_section(display);

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
        let status = drv::battery_renderer::BatteryStatus {
            level: self.battery_level,
            is_charging: true,
        };
        let _ = render_battery_status(display, &status);
    }

    fn draw_time_section<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        Ok(())
    }

    fn draw_weather_section<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let y_start = 240;
        let panel_width = 240;
        let gap = 40;

        // 温度面板
        // self.draw_temperature_panel(display, 50, y_start, panel_width);

        // 天气图标面板
        self.draw_weather_icon_panel(
            display,
            (50 + panel_width + gap) as i32,
            y_start,
            panel_width,
        );

        // // 湿度面板
        // self.draw_humidity_panel(
        //     display,
        //     (50 + 2 * (panel_width + gap)) as i32,
        //     y_start,
        //     panel_width,
        // );
    }

    fn draw_temperature_panel<D>(
        &self,
        display: &mut D,
        x: i32,
        y: i32,
        width: u32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        Ok(())
    }

    fn draw_weather_icon_panel<D>(&self, display: &mut D, x: i32, y: i32, width: u32)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let _ = draw_weather_icon(
            drv::weather_icons::WeatherIcon::sunny,
            Point::new(300, 300),
            display,
        );
    }

    fn draw_humidity_panel<D>(
        &self,
        display: &mut D,
        x: i32,
        y: i32,
        width: u32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        Ok(())
    }

    fn draw_quote_section<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        drv::hitokoto_renderer::render_next_hitokoto(display);
    }
}

// 使用示例
pub fn create_sample_display() -> InkDisplay {
    InkDisplay {
        time: "14:30".to_string(),
        date: "2024-01-15".to_string(),
        weekday: "星期一".to_string(),
        temperature: 23,
        humidity: 65,
        weather_condition: WeatherCondition::Sunny,
        battery_level: 85,
        wifi_connected: true,
        quote: "生活就像一盒巧克力，你永远不知道下一颗是什么味道。".to_string(),
        quote_author: "阿甘正传".to_string(),
    }
}

// 在主函数中使用
pub fn render_display<D>(display: &mut D)
where
    D: DrawTarget<Color = QuadColor>,
{
    let ink_display = create_sample_display();
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
