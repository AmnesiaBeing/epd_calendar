use alloc::{format, string::String};
use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::{
    assets::{
        generated_fonts::{FontSize, get_font_metrics},
        generated_hitokoto_data::{FROM_STRINGS, FROM_WHO_STRINGS},
    },
    common::Hitokoto,
    render::{TextRenderer, text_renderer::TextAlignment},
};

const HITOKOTO_RECT: Rectangle = Rectangle::new(Point::new(36, 400), Size::new(728, 80));

impl Drawable for &Hitokoto {
    type Color = QuadColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let hitokoto = (*self).clone();
        let content = hitokoto.hitokoto;
        let from = FROM_STRINGS[hitokoto.from as usize];
        let from_who = FROM_WHO_STRINGS[hitokoto.from_who as usize];

        // 创建格言渲染器
        let mut content_renderer = TextRenderer::new(FontSize::Medium, Point::new(0, 0));

        // 计算作者和来源行
        let author_source_line = if from_who.is_empty() && from.is_empty() {
            String::new()
        } else if from_who.is_empty() {
            format!("《{}》", from)
        } else if from.is_empty() {
            format!("——{}", from_who)
        } else {
            format!("——{}《{}》", from_who, from)
        };

        // 1. 计算格言行数
        let max_width = HITOKOTO_RECT.size.width as i32;
        let content_height = content_renderer.calculate_text_height(&content, max_width) as i32;
        let line_height = get_font_metrics(FontSize::Medium, false).1 as i32; // 使用半角字符的行高
        let content_lines = (content_height + line_height - 1) / line_height; // 向上取整

        // 2. 检查是否超过3行
        if content_lines > 3 {
            // 超过3行，只显示格言（居中），不显示作者和来源
            log::warn!("格言超过3行，不显示作者和来源: {}", content);

            // 计算垂直居中位置
            let total_height = content_height;
            let top_margin = (HITOKOTO_RECT.size.height as i32 - total_height) / 2;
            let start_y = HITOKOTO_RECT.top_left.y + top_margin;

            // 创建渲染器并绘制格言（居中）
            let mut renderer = TextRenderer::new(
                FontSize::Medium,
                Point::new(HITOKOTO_RECT.top_left.x, start_y),
            );
            renderer.draw_text_multiline(target, &content, max_width, TextAlignment::Center)?;

            return Ok(());
        }

        // 3. 计算整体垂直布局
        let author_source_height = if author_source_line.is_empty() {
            0
        } else {
            get_font_metrics(FontSize::Small, false).1 as i32 // 小字体行高
        };

        let line_spacing = 4; // 行间距
        let total_height = content_height + author_source_height + line_spacing;
        let top_margin = (HITOKOTO_RECT.size.height as i32 - total_height) / 2;

        // 4. 绘制格言
        let content_start_y = HITOKOTO_RECT.top_left.y + top_margin;
        content_renderer.move_to(Point::new(HITOKOTO_RECT.top_left.x, content_start_y));

        if content_lines == 1 {
            // 单行格言：居中显示
            content_renderer.draw_text_multiline(
                target,
                &content,
                max_width,
                TextAlignment::Center,
            )?;
        } else {
            // 2-3行格言：左对齐显示
            content_renderer.draw_text_multiline(
                target,
                &content,
                max_width,
                TextAlignment::Left,
            )?;
        }

        // 5. 绘制作者和来源（如果有）
        if !author_source_line.is_empty() {
            let author_start_y = content_start_y + content_height + line_spacing;
            let mut author_renderer = TextRenderer::new(
                FontSize::Small,
                Point::new(HITOKOTO_RECT.top_left.x, author_start_y),
            );

            // 居右显示作者和来源
            author_renderer.draw_text_right(target, &author_source_line, max_width)?;
        }

        Ok(())
    }
}
