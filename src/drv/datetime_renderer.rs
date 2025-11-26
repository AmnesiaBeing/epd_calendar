//! 时间渲染器 - 在屏幕指定位置渲染时间

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use epd_waveshare::color::QuadColor;

// 位置定义
const TIME_MARGIN_TOP: i32 = 10;
const TIME_FONT_HEIGHT: u32 = 120;
const TIME_CHAR_WIDTH: u32 = 60;
const DATE_MARGIN_TOP: u32 = 120 + 10 + 5;
const SCREEN_WIDTH: u32 = 800;
const SEGMENT_THICKNESS: u32 = 8;
const DIGIT_SPACING: i32 = 10; // 数字之间的间距
const COLON_SPACING: i32 = 5; // 冒号与数字之间的间距
const COLON_WIDTH: i32 = 10; // 冒号宽度

use crate::drv::generated_date_fonts::{
    DATE_FULL_WIDTH_FONT as FULL_WIDTH_FONT, DATE_HALF_WIDTH_FONT as HALF_WIDTH_FONT,
};

// 颜色定义
const TEXT_COLOR: QuadColor = QuadColor::Black;

/// AM/PM 状态枚举
#[derive(Debug, Clone, Copy)]
pub enum Meridiem {
    AM,
    PM,
}

/// 时间渲染配置
pub struct TimeConfig {
    /// 小时的十位数字 (0-2)
    pub hour_tens: u8,
    /// 小时的个位数字 (0-9)
    pub hour_ones: u8,
    /// 分钟的十位数字 (0-5)
    pub minute_tens: u8,
    /// 分钟的个位数字 (0-9)
    pub minute_ones: u8,
    /// 是否需要绘制 AM/PM 标识
    pub show_meridiem: bool,
    /// 当前是上午还是下午
    pub meridiem: Meridiem,
}

/// 日期渲染配置
pub struct DateConfig {
    /// 年
    pub year: u32,
    /// 月
    pub month: u8,
    /// 日
    pub day: u8,
    /// 星期
    pub weekday: u8,
}

/// 七段数码管段定义
///  a
/// f b
///  g
/// e c
///  d
#[derive(Debug, Clone, Copy)]
enum Segment {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

impl From<Segment> for usize {
    fn from(segment: Segment) -> Self {
        segment as usize
    }
}

// 获取数字对应的七段数码管显示段
fn get_segments_for_digit(digit: u8) -> [bool; 7] {
    match digit {
        0 => [true, true, true, true, true, true, false],
        1 => [false, true, true, false, false, false, false],
        2 => [true, true, false, true, true, false, true],
        3 => [true, true, true, true, false, false, true],
        4 => [false, true, true, false, false, true, true],
        5 => [true, false, true, true, false, true, true],
        6 => [true, false, true, true, true, true, true],
        7 => [true, true, true, false, false, false, false],
        8 => [true, true, true, true, true, true, true],
        9 => [true, true, true, true, false, true, true],
        _ => [false, false, false, false, false, false, false],
    }
}

// 绘制单个数字的七段数码管
fn draw_digit<D>(display: &mut D, digit: u8, position: Point, size: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let char_width = TIME_CHAR_WIDTH as i32;
    let char_height = size as i32;
    let thickness = SEGMENT_THICKNESS as i32;

    // 计算段的尺寸
    let long_length = char_width - thickness * 2;
    let short_length = (char_height - thickness * 3) / 2;

    let segments = get_segments_for_digit(digit);

    // 段 A: 顶部横线
    if segments[Segment::A as usize] {
        Line::new(
            Point::new(position.x + thickness, position.y),
            Point::new(position.x + thickness + long_length, position.y),
        )
        .into_styled(PrimitiveStyle::with_stroke(
            QuadColor::Black,
            thickness as u32,
        ))
        .draw(display)?;
    }

    // 段 B: 右上竖线
    if segments[Segment::B as usize] {
        Line::new(
            Point::new(position.x + char_width - thickness, position.y + thickness),
            Point::new(
                position.x + char_width - thickness,
                position.y + thickness + short_length,
            ),
        )
        .into_styled(PrimitiveStyle::with_stroke(
            QuadColor::Black,
            thickness as u32,
        ))
        .draw(display)?;
    }

    // 段 C: 右下竖线
    if segments[Segment::C as usize] {
        Line::new(
            Point::new(
                position.x + char_width - thickness,
                position.y + thickness * 2 + short_length,
            ),
            Point::new(
                position.x + char_width - thickness,
                position.y + thickness * 2 + short_length * 2,
            ),
        )
        .into_styled(PrimitiveStyle::with_stroke(
            QuadColor::Black,
            thickness as u32,
        ))
        .draw(display)?;
    }

    // 段 D: 底部横线
    if segments[Segment::D as usize] {
        Line::new(
            Point::new(position.x + thickness, position.y + char_height),
            Point::new(
                position.x + thickness + long_length,
                position.y + char_height,
            ),
        )
        .into_styled(PrimitiveStyle::with_stroke(
            QuadColor::Black,
            thickness as u32,
        ))
        .draw(display)?;
    }

    // 段 E: 左下竖线
    if segments[Segment::E as usize] {
        Line::new(
            Point::new(position.x, position.y + thickness * 2 + short_length),
            Point::new(position.x, position.y + thickness * 2 + short_length * 2),
        )
        .into_styled(PrimitiveStyle::with_stroke(
            QuadColor::Black,
            thickness as u32,
        ))
        .draw(display)?;
    }

    // 段 F: 左上竖线
    if segments[Segment::F as usize] {
        Line::new(
            Point::new(position.x, position.y + thickness),
            Point::new(position.x, position.y + thickness + short_length),
        )
        .into_styled(PrimitiveStyle::with_stroke(
            QuadColor::Black,
            thickness as u32,
        ))
        .draw(display)?;
    }

    // 段 G: 中间横线
    if segments[Segment::G as usize] {
        Line::new(
            Point::new(position.x + thickness, position.y + char_height / 2),
            Point::new(
                position.x + thickness + long_length,
                position.y + char_height / 2,
            ),
        )
        .into_styled(PrimitiveStyle::with_stroke(
            QuadColor::Black,
            thickness as u32,
        ))
        .draw(display)?;
    }

    Ok(())
}

// 绘制冒号
fn draw_colon<D>(display: &mut D, position: Point, size: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let dot_radius = size / 15;
    let center_y = position.y + (size as i32) / 2;

    // 上圆点
    Circle::new(
        Point::new(position.x, center_y - (size as i32) / 4),
        dot_radius,
    )
    .into_styled(PrimitiveStyle::with_fill(QuadColor::Black))
    .draw(display)?;

    // 下圆点
    Circle::new(
        Point::new(position.x, center_y + (size as i32) / 4),
        dot_radius,
    )
    .into_styled(PrimitiveStyle::with_fill(QuadColor::Black))
    .draw(display)?;

    Ok(())
}

// 绘制 AM/PM 标识
fn draw_meridiem<D>(
    display: &mut D,
    meridiem: Meridiem,
    position: Point,
    size: u32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let box_width = size / 4;
    let box_height = size / 6;

    let style = PrimitiveStyle::with_fill(QuadColor::Black);

    // 绘制背景框
    Rectangle::new(position, Size::new(box_width, box_height))
        .into_styled(style)
        .draw(display)?;

    Ok(())
}

// 时间渲染函数
pub fn render_time<D>(display: &mut D, config: &TimeConfig) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    // 计算时间显示部分的总宽度（不考虑AM/PM）
    // 4个数字 + 3个数字间距 + 1个冒号 + 2个冒号间距
    let time_display_width = (4 * TIME_CHAR_WIDTH as i32) +  // 4个数字
        (3 * DIGIT_SPACING) +           // 3个数字间距
        COLON_WIDTH +                   // 冒号宽度
        (2 * COLON_SPACING); // 2个冒号间距

    // 计算居中起始位置
    let start_x = (SCREEN_WIDTH as i32 - time_display_width) / 2;
    let start_y = TIME_MARGIN_TOP;

    let mut current_x = start_x;

    // 绘制小时十位数字
    draw_digit(
        display,
        config.hour_tens,
        Point::new(current_x, start_y),
        TIME_FONT_HEIGHT,
    )?;
    current_x += TIME_CHAR_WIDTH as i32 + DIGIT_SPACING;

    // 绘制小时个位数字
    draw_digit(
        display,
        config.hour_ones,
        Point::new(current_x, start_y),
        TIME_FONT_HEIGHT,
    )?;
    current_x += TIME_CHAR_WIDTH as i32 + COLON_SPACING;

    // 绘制冒号
    draw_colon(display, Point::new(current_x, start_y), TIME_FONT_HEIGHT)?;
    current_x += COLON_WIDTH + COLON_SPACING;

    // 绘制分钟十位数字
    draw_digit(
        display,
        config.minute_tens,
        Point::new(current_x, start_y),
        TIME_FONT_HEIGHT,
    )?;
    current_x += TIME_CHAR_WIDTH as i32 + DIGIT_SPACING;

    // 绘制分钟个位数字
    draw_digit(
        display,
        config.minute_ones,
        Point::new(current_x, start_y),
        TIME_FONT_HEIGHT,
    )?;

    // 绘制 AM/PM 标识（如果需要）
    if config.show_meridiem {
        // AM/PM指示器放在时间显示部分的右侧，不影响居中
        let am_pm_x = current_x + TIME_CHAR_WIDTH as i32 + COLON_SPACING;
        draw_meridiem(
            display,
            config.meridiem,
            Point::new(am_pm_x, start_y + TIME_FONT_HEIGHT as i32 / 4),
            TIME_FONT_HEIGHT,
        )?;
    }

    Ok(())
}

// 智能文本渲染器
pub struct HitokotoTextRenderer {
    full_width_style: MonoTextStyle<'static, QuadColor>,
    half_width_style: MonoTextStyle<'static, QuadColor>,
    current_x: i32,
    current_y: i32,
    line_height: i32,
}

impl HitokotoTextRenderer {
    pub fn new(position: Point) -> Self {
        Self {
            full_width_style: MonoTextStyle::new(&FULL_WIDTH_FONT, TEXT_COLOR),
            half_width_style: MonoTextStyle::new(&HALF_WIDTH_FONT, TEXT_COLOR),
            current_x: position.x,
            current_y: position.y,
            line_height: FULL_WIDTH_FONT.character_size.height as i32,
        }
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
            log::info!(
                "draw_text: char={}, is_half={}, start_x={}, start_y={}, text={}",
                c,
                Self::is_half_width_char(c),
                self.current_x,
                self.current_y,
                text
            );
            if Self::is_half_width_char(c) {
                // 半角字符
                Text::new(
                    &c.to_string(),
                    Point::new(self.current_x, self.current_y),
                    self.half_width_style,
                )
                .draw(display)?;
                self.current_x += HALF_WIDTH_FONT.character_size.width as i32;
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

    // 渲染右对齐文本
    pub fn draw_text_right<D>(
        &mut self,
        display: &mut D,
        text: &str,
        right_x: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let text_width = Self::calculate_text_width(text) as i32;
        let start_x = right_x - text_width;

        let temp_x = self.current_x;
        self.current_x = start_x;
        let result = self.draw_text(display, text);
        self.current_x = temp_x;

        result
    }

    // 渲染居中对齐文本
    pub fn draw_text_centered<D>(
        &mut self,
        display: &mut D,
        text: &str,
        center_x: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let text_width = Self::calculate_text_width(text) as i32;
        let start_x = center_x - text_width / 2;

        let temp_x = self.current_x;
        self.current_x = start_x;
        let result = self.draw_text(display, text);
        self.current_x = temp_x;

        result
    }

    // 计算文本宽度
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

    // 移动到指定位置
    pub fn move_to(&mut self, position: Point) {
        self.current_x = position.x;
        self.current_y = position.y;
    }
}

// 显示区域配置
const DISPLAY_AREA: Rectangle = Rectangle::new(Point::new(0, 120), Size::new(800, 40));

// 日期渲染函数
fn render_date<D>(display: &mut D, config: &DateConfig) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let mut renderer = HitokotoTextRenderer::new(Point::new(
        DISPLAY_AREA.top_left.x,
        DISPLAY_AREA.top_left.y + (DISPLAY_AREA.size.height as i32) / 2, // 垂直居中
    ));

    let center_x = DISPLAY_AREA.top_left.x + (DISPLAY_AREA.size.width as i32) / 2;
    renderer.draw_text_centered(display, &"2025-11-26 周三", center_x)?;

    Ok(())
}

// 日期时间渲染函数
pub fn render_datetime<D>(
    display: &mut D,
    date_config: &DateConfig,
    time_config: &TimeConfig,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    render_date(display, date_config)?;
    render_time(display, time_config)?;
    Ok(())
}
