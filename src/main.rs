use epd_waveshare::color::QuadColor;
use epd_waveshare::prelude::WaveshareDisplay;
use log::info;

use embassy_executor::Spawner;

use embedded_graphics::{
    mono_font::MonoTextStyle,
    prelude::*,
    primitives::{Circle, CornerRadii, Line, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

mod app;
mod bsp;

// 使用您已有的字体
use crate::app::hitokoto_fonts::{FULL_WIDTH_FONT, HALF_WIDTH_FONT};

// 颜色定义
const BACKGROUND_COLOR: QuadColor = QuadColor::White;
const TEXT_COLOR: QuadColor = QuadColor::Black;
const PANEL_BG_COLOR: QuadColor = QuadColor::White;
const PANEL_TEXT_COLOR: QuadColor = QuadColor::Black;

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
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 1. 清屏
        self.clear_screen(display)?;

        // 2. 绘制顶部状态栏
        self.draw_status_bar(display)?;

        // 3. 绘制主要时间区域
        self.draw_time_section(display)?;

        // 4. 绘制天气信息
        self.draw_weather_section(display)?;

        // 5. 绘制格言区域
        self.draw_quote_section(display)?;

        // 6. 绘制底部装饰线
        self.draw_decoration(display)?;

        Ok(())
    }

    fn clear_screen<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        Rectangle::new(Point::new(0, 0), Size::new(800, 480))
            .into_styled(PrimitiveStyle::with_fill(BACKGROUND_COLOR))
            .draw(display)
    }

    fn draw_status_bar<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let style = MonoTextStyle::new(&HALF_WIDTH_FONT, TEXT_COLOR);

        // 左侧：Wi-Fi状态
        let wifi_text = if self.wifi_connected {
            "Wi-Fi ●"
        } else {
            "Wi-Fi ○"
        };
        Text::new(wifi_text, Point::new(20, 20), style).draw(display)?;

        // 中间：日期和星期
        let date_style = MonoTextStyle::new(&FULL_WIDTH_FONT, TEXT_COLOR);
        let date_text = format!("{} {}", self.date, self.weekday);
        Text::new(&date_text, Point::new(300, 20), date_style).draw(display)?;

        // 右侧：电池电量
        let battery_text = format!("电池 {}%", self.battery_level);
        Text::new(&battery_text, Point::new(650, 20), style).draw(display)?;

        // 分隔线
        Line::new(Point::new(0, 40), Point::new(800, 40))
            .into_styled(PrimitiveStyle::with_stroke(TEXT_COLOR, 1))
            .draw(display)?;

        Ok(())
    }

    fn draw_time_section<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 时间显示区域背景
        RoundedRectangle::new(
            Rectangle::new(Point::new(50, 60), Size::new(700, 150)),
            CornerRadii::new(Size::new(20, 20)),
        )
        .into_styled(PrimitiveStyle::with_fill(PANEL_BG_COLOR))
        .draw(display)?;

        // 时间文字
        let time_style = MonoTextStyle::new(&FULL_WIDTH_FONT, PANEL_TEXT_COLOR);
        Text::new(&self.time, Point::new(400, 140), time_style).draw(display)?;

        Ok(())
    }

    fn draw_weather_section<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let y_start = 240;
        let panel_width = 240;
        let gap = 40;

        // 温度面板
        self.draw_temperature_panel(display, 50, y_start, panel_width)?;

        // 天气图标面板
        self.draw_weather_icon_panel(display, (50 + panel_width + gap).try_into().unwrap(), y_start, panel_width)?;

        // 湿度面板
        self.draw_humidity_panel(display, (50 + 2 * (panel_width + gap)).try_into().unwrap(), y_start, panel_width)?;

        Ok(())
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
        let panel = RoundedRectangle::new(
            Rectangle::new(Point::new(x, y), Size::new(width, 100)),
            CornerRadii::new(Size::new(10, 10)),
        )
        .into_styled(PrimitiveStyle::with_fill(PANEL_BG_COLOR));
        panel.draw(display)?;

        let temp_style = MonoTextStyle::new(&FULL_WIDTH_FONT, PANEL_TEXT_COLOR);
        let label_style = MonoTextStyle::new(&HALF_WIDTH_FONT, PANEL_TEXT_COLOR);

        Text::new("温度", Point::new(x + 20, y + 25), label_style).draw(display)?;

        let temp_text = format!("{}°C", self.temperature);
        Text::new(&temp_text, Point::new(x + 20, y + 65), temp_style).draw(display)?;

        Ok(())
    }

    fn draw_weather_icon_panel<D>(
        &self,
        display: &mut D,
        x: i32,
        y: i32,
        width: u32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let panel = RoundedRectangle::new(
            Rectangle::new(Point::new(x, y), Size::new(width, 100)),
            CornerRadii::new(Size::new(10, 10)),
        )
        .into_styled(PrimitiveStyle::with_fill(PANEL_BG_COLOR));
        panel.draw(display)?;

        let style = MonoTextStyle::new(&FULL_WIDTH_FONT, PANEL_TEXT_COLOR);
        let small_style = MonoTextStyle::new(&HALF_WIDTH_FONT, PANEL_TEXT_COLOR);

        // 天气图标（用文字符号表示）
        let weather_icon = match self.weather_condition {
            WeatherCondition::Sunny => "☀",
            WeatherCondition::Cloudy => "☁",
            WeatherCondition::Rainy => "🌧",
            WeatherCondition::Snowy => "❄",
            WeatherCondition::Foggy => "🌫",
        };

        Text::new(weather_icon, Point::new(x + 30, y + 30), style).draw(display)?;

        let condition_text = match self.weather_condition {
            WeatherCondition::Sunny => "晴朗",
            WeatherCondition::Cloudy => "多云",
            WeatherCondition::Rainy => "有雨",
            WeatherCondition::Snowy => "下雪",
            WeatherCondition::Foggy => "有雾",
        };
        Text::new(condition_text, Point::new(x + 30, y + 70),small_style).draw(display)?;

        Ok(())
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
        let panel = RoundedRectangle::new(
            Rectangle::new(Point::new(x, y), Size::new(width, 100)),
            CornerRadii::new(Size::new(10, 10)),
        )
        .into_styled(PrimitiveStyle::with_fill(PANEL_BG_COLOR));
        panel.draw(display)?;

        let style = MonoTextStyle::new(&FULL_WIDTH_FONT, PANEL_TEXT_COLOR);
        let small_style = MonoTextStyle::new(&HALF_WIDTH_FONT, PANEL_TEXT_COLOR);

        Text::new("湿度", Point::new(x + 20, y + 25), small_style).draw(display)?;

        let humidity_text = format!("{}%", self.humidity);
        Text::new(&humidity_text, Point::new(x + 20, y + 65), style).draw(display)?;

        // 湿度进度条
        let bar_width = (width - 40) as i32;
        let fill_width = (bar_width * self.humidity as i32 / 100) as u32;

        // 背景条
        RoundedRectangle::new(
            Rectangle::new(Point::new(x + 20, y + 80), Size::new(bar_width as u32, 8)),
            CornerRadii::new(Size::new(4, 4)),
        )
        .into_styled(PrimitiveStyle::with_fill(BACKGROUND_COLOR))
        .draw(display)?;

        // 填充条
        if fill_width > 0 {
            RoundedRectangle::new(
                Rectangle::new(Point::new(x + 20, y + 80), Size::new(fill_width, 8)),
                CornerRadii::new(Size::new(4, 4)),
            )
            .into_styled(PrimitiveStyle::with_fill(PANEL_TEXT_COLOR))
            .draw(display)?;
        }

        Ok(())
    }

    fn draw_quote_section<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let y_start = 360;

        // 格言面板
        RoundedRectangle::new(
            Rectangle::new(Point::new(50, y_start), Size::new(700, 80)),
            CornerRadii::new(Size::new(15, 15)),
        )
        .into_styled(PrimitiveStyle::with_fill(PANEL_BG_COLOR))
        .draw(display)?;

        let quote_style = MonoTextStyle::new(&FULL_WIDTH_FONT, PANEL_TEXT_COLOR);
        let author_style = MonoTextStyle::new(&HALF_WIDTH_FONT, PANEL_TEXT_COLOR);

        // 格言内容（简单截断处理）
        let display_quote = if self.quote.len() > 20 {
            format!("{}...", &self.quote[..20])
        } else {
            self.quote.clone()
        };

        Text::new(&display_quote, Point::new(70, y_start + 30), quote_style).draw(display)?;

        if !self.quote_author.is_empty() {
            let author_text = format!("—— {}", self.quote_author);
            Text::new(&author_text, Point::new(650, y_start + 60), author_style).draw(display)?;
        }

        Ok(())
    }

    fn draw_decoration<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 底部装饰线
        Line::new(Point::new(100, 470), Point::new(700, 470))
            .into_styled(PrimitiveStyle::with_stroke(TEXT_COLOR, 2))
            .draw(display)?;

        // 装饰点
        for i in 0..5 {
            let x = 150 + i * 100;
            Circle::new(Point::new(x, 470), 3)
                .into_styled(PrimitiveStyle::with_fill(TEXT_COLOR))
                .draw(display)?;
        }

        Ok(())
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
        quote: "你好世界".to_string(),
        quote_author: "阿甘正传".to_string(),
    }
}

// 在主函数中使用
pub fn render_display<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let ink_display = create_sample_display();
    ink_display.draw(display)
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

    render_display(&mut board.epd_display).unwrap();

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
