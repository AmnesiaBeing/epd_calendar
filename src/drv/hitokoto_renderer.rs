//! 格言渲染器 - 在屏幕固定区域渲染格言文本

use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::Text;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    mono_font::MonoTextStyle,
};
use epd_waveshare::color::QuadColor;

use crate::drv::hitokoto_data::{FROM_STRINGS, FROM_WHO_STRINGS, HITOKOTOS, Hitokoto};
use crate::drv::hitokoto_fonts::{
    HITOKOTO_FULL_WIDTH_FONT as FULL_WIDTH_FONT, HITOKOTO_HALF_WIDTH_FONT as HALF_WIDTH_FONT,
};

// 颜色定义
const TEXT_COLOR: QuadColor = QuadColor::Black;

// 边距配置
const MARGIN_X: i32 = 36;
const LINE_SPACING: i32 = 2;

// 显示区域配置
const DISPLAY_AREA: Rectangle = Rectangle::new(
    Point::new(0, 360),
    Size::new(800, 120), // 480-360=120
);

// 错误类型
pub enum HitokotoError {
    TooLong, // 格言过长，无法在指定区域内显示
}

// 格言布局信息
pub struct HitokotoLayout {
    pub content_lines: Vec<String>,
    pub author_line: String,
    pub total_lines: usize,
}

impl HitokotoLayout {
    /// 计算格言布局
    pub fn calculate(hitokoto: &Hitokoto) -> Result<Self, HitokotoError> {
        let content = &hitokoto.hitokoto;
        let from = FROM_STRINGS[hitokoto.from];
        let from_who = if hitokoto.from_who != usize::MAX {
            FROM_WHO_STRINGS[hitokoto.from_who]
        } else {
            "佚名"
        };

        // 生成作者行
        let author_line = if from_who == "佚名" {
            format!("——{}", from)
        } else {
            format!("——{}《{}》", from_who, from)
        };

        // 计算可用宽度（减去边距）
        let available_width = (DISPLAY_AREA.size.width as i32 - 2 * MARGIN_X) as u32;

        // 换行格言内容
        let content_lines = Self::wrap_text(content, available_width);
        let total_lines = content_lines.len() + 1; // 内容行数 + 作者行

        // 检查是否超出显示区域
        let max_lines = Self::calculate_max_lines();
        if total_lines > max_lines {
            return Err(HitokotoError::TooLong);
        }

        Ok(Self {
            content_lines,
            author_line,
            total_lines,
        })
    }

    // 文本换行函数
    fn wrap_text(text: &str, max_width: u32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut line_width = 0;

        for c in text.chars() {
            let char_width = if HitokotoTextRenderer::is_half_width_char(c) {
                FULL_WIDTH_FONT.character_size.width / 2
            } else {
                FULL_WIDTH_FONT.character_size.width
            };

            // 检查是否需要换行
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

    // 计算最大可显示行数
    fn calculate_max_lines() -> usize {
        let line_height = FULL_WIDTH_FONT.character_size.height as i32 + LINE_SPACING;
        let available_height = DISPLAY_AREA.size.height as i32;
        (available_height / line_height) as usize
    }

    /// 获取总高度（像素）
    pub fn total_height(&self) -> u32 {
        let line_height = FULL_WIDTH_FONT.character_size.height as u32;
        let spacing = LINE_SPACING as u32;
        (self.total_lines as u32) * line_height + ((self.total_lines - 1) as u32) * spacing
    }

    /// 计算垂直居中的起始Y坐标
    pub fn calculate_vertical_center_start(&self) -> i32 {
        let total_height = self.total_height() as i32;
        let available_height = DISPLAY_AREA.size.height as i32;
        DISPLAY_AREA.top_left.y + (available_height - total_height) / 2
    }
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
            line_height: FULL_WIDTH_FONT.character_size.height as i32 + LINE_SPACING,
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

        log::info!(
            "draw_text: start_x={}, start_y={}, text={}",
            start_x,
            self.current_y,
            text
        );

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

// 格言渲染器
pub struct HitokotoRenderer {
    rng: crate::drv::lcg::Lcg,
}

impl HitokotoRenderer {
    pub fn new() -> Self {
        Self {
            rng: crate::drv::lcg::Lcg::new(),
        }
    }

    /// 渲染格言到显示设备
    pub fn render<D>(&mut self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        const MAX_RETRIES: usize = 3; // 最大重试次数

        for attempt in 0..MAX_RETRIES {
            let index = self.rng.next_index(HITOKOTOS.len());
            let hitokoto = &HITOKOTOS[index];

            match HitokotoLayout::calculate(hitokoto) {
                Ok(layout) => {
                    // 成功找到合适的格言，进行渲染
                    self.draw_hitokoto(display, &layout)?;
                    return Ok(());
                }
                Err(HitokotoError::TooLong) => {
                    // 记录过长的格言
                    log::warn!(
                        "Hitokoto too long (attempt {}): index={}, content='{}'",
                        attempt + 1,
                        index,
                        hitokoto.hitokoto
                    );

                    if attempt == MAX_RETRIES - 1 {
                        // 最后一次尝试也失败了，使用默认格言
                        log::error!(
                            "Failed to find suitable hitokoto after {} attempts",
                            MAX_RETRIES
                        );
                        self.render_fallback(display)?;
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    /// 渲染特定的格言布局
    fn draw_hitokoto<D>(&self, display: &mut D, layout: &HitokotoLayout) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 使用垂直居中布局
        let start_y = layout.calculate_vertical_center_start();
        let mut renderer = HitokotoTextRenderer::new(Point::new(MARGIN_X, start_y));

        let center_x = DISPLAY_AREA.size.width as i32 / 2;
        let right_x = DISPLAY_AREA.size.width as i32 - MARGIN_X;

        log::info!(
            "render_hitokoto: start_y={}, center_x={}, right_x={}",
            start_y,
            center_x,
            right_x
        );

        // 根据行数选择布局策略
        match layout.content_lines.len() {
            1 => {
                // 单行格言：内容居中，作者右对齐
                renderer.draw_text_centered(display, &layout.content_lines[0], center_x)?;
                renderer.draw_text_right(display, &layout.author_line, right_x)?;
            }
            _ => {
                // 多行格言：内容左对齐，作者右对齐在最后一行
                for line in &layout.content_lines {
                    renderer.draw_text(display, line)?;
                }
                // 调整作者行位置，使其与内容有适当间隔
                renderer.move_to(Point::new(
                    renderer.current_x,
                    renderer.current_y + LINE_SPACING * 2,
                ));
                renderer.draw_text_right(display, &layout.author_line, right_x)?;
            }
        }

        log::info!(
            "render_hitokoto: layout.content_lines={:?}",
            layout.content_lines
        );

        Ok(())
    }

    /// 渲染后备格言（当所有尝试都失败时使用）
    fn render_fallback<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let fallback_text = "格言加载中...";
        let mut renderer = HitokotoTextRenderer::new(Point::new(
            DISPLAY_AREA.top_left.x + MARGIN_X,
            DISPLAY_AREA.top_left.y + (DISPLAY_AREA.size.height as i32) / 2, // 垂直居中
        ));

        let center_x = DISPLAY_AREA.top_left.x + (DISPLAY_AREA.size.width as i32) / 2;
        renderer.draw_text_centered(display, fallback_text, center_x)?;

        Ok(())
    }
}

// 便捷函数：渲染下一句格言
pub fn render_next_hitokoto<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let mut renderer = HitokotoRenderer::new();
    renderer.render(display)
}
