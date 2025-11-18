// 该代码用于在固定位置渲染格言文本

use crate::drv::fonts::{FULL_WIDTH_FONT, HALF_WIDTH_FONT};

use epd_waveshare::color::QuadColor;

use embedded_graphics::{mono_font::MonoTextStyle, prelude::*, text::Text};

// 颜色定义
const BACKGROUND_COLOR: QuadColor = QuadColor::White;
const TEXT_COLOR: QuadColor = QuadColor::Black;

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

// 文本换行辅助函数
fn wrap_hitokoto_text(text: &str, max_width: u32) -> Vec<String> {
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

// 实际绘制格言文本的函数
fn draw_hitokoto_text<D>(display: &mut D, text: &str, position: Point) -> Result<Point, D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let mut renderer = SmartTextRenderer::new(position);

    // 显示的格言包括 格言 、 作者 和 来源 3个部分，其中 来源、作者 一定占据1行，需要提前计算格言占用的行数
    // 对于一行的格言，直接居中显示
    // 对于多行的格言，第一行填满，第二行左对齐
    // 作者来源需要右对齐显示
    // 对于一、二行的格言，作者来源显示在下一行
    // 对于三、四行的格言，作者来源显示在下半行
    // 对于五行的格言，不显示作者来源
    // 对于六行的格言，返回失败，让上层应用显示下一条格言，并将日志打印出来
    let lines = wrap_hitokoto_text(text, 800 - 36 - 36); // 800 是屏幕宽度，36 是距离屏幕两边的间隔
    for line in lines {
        renderer.draw_centered_text(display, &line)?;
    }

    Ok(renderer.current_position())
}

// 绘制格言文本的入口函数
pub fn next_hitokoto<D>(display: &mut D)
where
    D: DrawTarget<Color = QuadColor>,
{
    use super::hitokoto_data::{FROM_STRINGS, HITOKOTOS, Hitokoto};
    let mut rng = super::lcg::Lcg::new();
    let index = rng.next_index(HITOKOTOS.len());
    let hitokoto: &Hitokoto = &HITOKOTOS[index];
    let from = &FROM_STRINGS[hitokoto.from];
    let from_who = &FROM_STRINGS[hitokoto.from_who];
}
