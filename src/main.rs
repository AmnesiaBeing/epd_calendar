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

mod app;
mod bsp;

// 使用您已有的字体
use crate::app::{
    fonts::{FULL_WIDTH_FONT, HALF_WIDTH_FONT},
    weather_icons::{WeatherIcon, get_icon_data, get_icon_image_raw},
};

// 颜色定义
const BACKGROUND_COLOR: QuadColor = QuadColor::White;
const TEXT_COLOR: QuadColor = QuadColor::Black;
const PANEL_BG_COLOR: QuadColor = QuadColor::White;
const PANEL_TEXT_COLOR: QuadColor = QuadColor::Black;

// 智能文本渲染器
pub struct SmartTextRenderer {
    full_width_style: MonoTextStyle<'static, QuadColor>,
    half_width_style: MonoTextStyle<'static, QuadColor>,
    current_x: i32,
    current_y: i32,
    line_height: i32,
}

impl SmartTextRenderer {
    pub fn new(position: Point) -> Self {
        Self {
            full_width_style: MonoTextStyle::new(&FULL_WIDTH_FONT, TEXT_COLOR),
            half_width_style: MonoTextStyle::new(&HALF_WIDTH_FONT, TEXT_COLOR),
            current_x: position.x,
            current_y: position.y,
            line_height: FULL_WIDTH_FONT.character_size.height as i32 + 2,
        }
    }

    pub fn with_color(mut self, color: QuadColor) -> Self {
        self.full_width_style = MonoTextStyle::new(&FULL_WIDTH_FONT, color);
        self.half_width_style = MonoTextStyle::new(&HALF_WIDTH_FONT, color);
        self
    }

    pub fn with_line_height(mut self, line_height: i32) -> Self {
        self.line_height = line_height;
        self
    }

    // 判断字符是否为半角字符
    fn is_half_width_char(c: char) -> bool {
        c.is_ascii() && !c.is_ascii_control()
    }

    // 渲染单行文本（自动处理全角半角混合）
    pub fn draw_text<D>(&mut self, display: &mut D, text: &str) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let start_x = self.current_x;

        for c in text.chars() {
            if Self::is_half_width_char(c) {
                // 半角字符
                Text::new(
                    &c.to_string(),
                    Point::new(self.current_x, self.current_y),
                    self.half_width_style,
                )
                .draw(display)?;
                self.current_x += (FULL_WIDTH_FONT.character_size.width / 2) as i32;
            } else {
                // 全角字符
                Text::new(
                    &c.to_string(),
                    Point::new(self.current_x, self.current_y),
                    self.full_width_style,
                )
                .draw(display)?;
                self.current_x += FULL_WIDTH_FONT.character_size.width as i32;
            }
        }

        // 移动到下一行
        self.current_x = start_x;
        self.current_y += self.line_height;

        Ok(())
    }

    // 渲染文本并限制最大宽度（自动换行）
    pub fn draw_text_wrapped<D>(
        &mut self,
        display: &mut D,
        text: &str,
        max_width: u32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let start_x = self.current_x;
        let mut line_width = 0;
        let mut current_line = String::new();

        for c in text.chars() {
            let char_width = if Self::is_half_width_char(c) {
                (FULL_WIDTH_FONT.character_size.width / 2) as i32
            } else {
                FULL_WIDTH_FONT.character_size.width as i32
            };

            // 检查是否需要换行
            if line_width + char_width > max_width as i32 && !current_line.is_empty() {
                // 绘制当前行
                self.draw_text(display, &current_line)?;
                current_line.clear();
                line_width = 0;
            }

            current_line.push(c);
            line_width += char_width;
        }

        // 绘制最后一行
        if !current_line.is_empty() {
            self.draw_text(display, &current_line)?;
        }

        self.current_x = start_x;
        Ok(())
    }

    // 移动到指定位置
    pub fn move_to(&mut self, position: Point) {
        self.current_x = position.x;
        self.current_y = position.y;
    }

    // 相对移动
    pub fn move_by(&mut self, dx: i32, dy: i32) {
        self.current_x += dx;
        self.current_y += dy;
    }

    // 获取当前位置
    pub fn current_position(&self) -> Point {
        Point::new(self.current_x, self.current_y)
    }

    // 计算文本宽度（用于居中计算）
    pub fn calculate_text_width(text: &str) -> u32 {
        let mut width = 0;
        for c in text.chars() {
            if Self::is_half_width_char(c) {
                width += FULL_WIDTH_FONT.character_size.width / 2;
            } else {
                width += FULL_WIDTH_FONT.character_size.width;
            }
        }
        width
    }

    // 创建居中对齐的文本渲染器
    pub fn centered_at(position: Point, container_width: u32) -> Self {
        let mut renderer = Self::new(position);
        renderer.current_x = position.x + (container_width / 2) as i32;
        renderer
    }

    // 绘制居中对齐的文本
    pub fn draw_centered_text<D>(&mut self, display: &mut D, text: &str) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let text_width = Self::calculate_text_width(text);
        let start_x = self.current_x - (text_width as i32 / 2);

        let temp_x = self.current_x;
        self.current_x = start_x;
        let result = self.draw_text(display, text);
        self.current_x = temp_x;

        result
    }
}

// 简化的文本样式
pub struct TextStyle {
    pub color: QuadColor,
    pub is_centered: bool,
    pub max_width: Option<u32>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: TEXT_COLOR,
            is_centered: false,
            max_width: None,
        }
    }
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_color(mut self, color: QuadColor) -> Self {
        self.color = color;
        self
    }

    pub fn centered(mut self) -> Self {
        self.is_centered = true;
        self
    }

    pub fn with_max_width(mut self, max_width: u32) -> Self {
        self.max_width = Some(max_width);
        self
    }
}

// 简化的文本绘制函数
pub fn draw_smart_text<D>(
    display: &mut D,
    text: &str,
    position: Point,
    style: TextStyle,
) -> Result<Point, D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let mut renderer = SmartTextRenderer::new(position).with_color(style.color);

    if let Some(max_width) = style.max_width {
        if style.is_centered {
            // 对于居中的换行文本，需要特殊处理
            let lines = wrap_text(text, max_width);
            for line in lines {
                renderer.draw_centered_text(display, &line)?;
            }
        } else {
            renderer.draw_text_wrapped(display, text, max_width)?;
        }
    } else if style.is_centered {
        renderer.draw_centered_text(display, text)?;
    } else {
        renderer.draw_text(display, text)?;
    }

    Ok(renderer.current_position())
}

// 文本换行辅助函数
fn wrap_text(text: &str, max_width: u32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut line_width = 0;

    for c in text.chars() {
        let char_width = if SmartTextRenderer::is_half_width_char(c) {
            FULL_WIDTH_FONT.character_size.width / 2
        } else {
            FULL_WIDTH_FONT.character_size.width
        };

        if line_width + char_width > max_width && !current_line.is_empty() {
            lines.push(current_line.clone());
            current_line.clear();
            line_width = 0;
        }

        current_line.push(c);
        line_width += char_width;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

pub fn draw_weather_icon<D>(
    icon: WeatherIcon,
    position: Point,
    display: &mut D,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let image_data = get_icon_data(icon);
    let width = 64;
    let height = 64;

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
        // self.draw_status_bar(display);

        // 3. 绘制时间
        // self.draw_time_section(display);

        // 4. 绘制农历
        // self.draw_time_section(display);

        // 5. 绘制天气信息
        self.draw_weather_section(display);

        // 6. 绘制格言区域
        // self.draw_quote_section(display);
    }

    fn clear_screen<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let _ = Rectangle::new(Point::new(0, 0), Size::new(800, 480))
            .into_styled(PrimitiveStyle::with_fill(BACKGROUND_COLOR))
            .draw(display);

        let _ = Line::new(Point { x: 10, y: 160 }, Point { x: 790, y: 160 })
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1))
            .draw(display);

        let _ = Line::new(Point { x: 10, y: 360 }, Point { x: 790, y: 360 })
            .into_styled(PrimitiveStyle::with_stroke(QuadColor::Black, 1))
            .draw(display);
    }

    fn draw_status_bar<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 左侧：Wi-Fi状态
        let wifi_text = if self.wifi_connected {
            "Wi-Fi ●"
        } else {
            "Wi-Fi ○"
        };
        let _ = draw_smart_text(display, wifi_text, Point::new(20, 20), TextStyle::new());

        // 右侧：电池电量 & 充电状态
        let battery_text = format!("电池 {}%", self.battery_level);
        let battery_width = SmartTextRenderer::calculate_text_width(&battery_text);
        let _ = draw_smart_text(
            display,
            &battery_text,
            Point::new(800 - 20 - battery_width as i32, 20),
            TextStyle::new(),
        );
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

        // 时间文字（居中）
        draw_smart_text(
            display,
            &self.time,
            Point::new(400, 140),
            TextStyle::new().with_color(PANEL_TEXT_COLOR).centered(),
        )?;

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
        let panel = RoundedRectangle::new(
            Rectangle::new(Point::new(x, y), Size::new(width, 100)),
            CornerRadii::new(Size::new(10, 10)),
        )
        .into_styled(PrimitiveStyle::with_fill(PANEL_BG_COLOR));
        panel.draw(display)?;

        draw_smart_text(
            display,
            "温度",
            Point::new(x + 20, y + 25),
            TextStyle::new().with_color(PANEL_TEXT_COLOR),
        )?;

        let temp_text = format!("{}°C", self.temperature);
        draw_smart_text(
            display,
            &temp_text,
            Point::new(x + 20, y + 65),
            TextStyle::new().with_color(PANEL_TEXT_COLOR),
        )?;

        Ok(())
    }

    fn draw_weather_icon_panel<D>(&self, display: &mut D, x: i32, y: i32, width: u32)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let panel = RoundedRectangle::new(
            Rectangle::new(Point::new(x, y), Size::new(width, 100)),
            CornerRadii::new(Size::new(10, 10)),
        )
        .into_styled(PrimitiveStyle::with_fill(PANEL_BG_COLOR));
        let _ = panel.draw(display);

        let _ = draw_weather_icon(WeatherIcon::sunny, Point::new(300, 300), display);
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

        draw_smart_text(
            display,
            "湿度",
            Point::new(x + 20, y + 25),
            TextStyle::new().with_color(PANEL_TEXT_COLOR),
        )?;

        let humidity_text = format!("{}%", self.humidity);
        draw_smart_text(
            display,
            &humidity_text,
            Point::new(x + 20, y + 65),
            TextStyle::new().with_color(PANEL_TEXT_COLOR),
        )?;

        // 湿度进度条（保持不变）
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

    fn draw_quote_section<D>(&self, display: &mut D)
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let y_start = 380;

        // 格言内容（自动换行）
        let _ = draw_smart_text(
            display,
            &self.quote,
            Point::new(70, y_start + 25),
            TextStyle::new()
                .with_color(PANEL_TEXT_COLOR)
                .with_max_width(660), // 700 - 40 (左右边距)
        );

        // 作者信息
        if !self.quote_author.is_empty() {
            let author_text = format!("—— {}", self.quote_author);
            let author_width = SmartTextRenderer::calculate_text_width(&author_text);
            let _ = draw_smart_text(
                display,
                &author_text,
                Point::new(750 - author_width as i32, y_start + 60),
                TextStyle::new().with_color(PANEL_TEXT_COLOR),
            );
        }
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
