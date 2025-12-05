use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use embedded_graphics::{
    Drawable,
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;
use log::warn;

use crate::{
    assets::{
        generated_fonts::FontSize,
        generated_hitokoto_data::{FROM_STRINGS, FROM_WHO_STRINGS},
    },
    common::Hitokoto,
    render::{TextRenderer, text_renderer::TextAlignment},
};

// 格言显示区域定义
pub const HITOKOTO_RECT: Rectangle = Rectangle::new(Point::new(36, 380), Size::new(728, 80));

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

        // 构建作者和来源行文本
        let author_source_line = if from_who.is_empty() && from.is_empty() {
            String::new()
        } else if from_who.is_empty() {
            format!("《{}》", from)
        } else if from.is_empty() {
            format!("——{}", from_who)
        } else {
            format!("——{}《{}》", from_who, from)
        };

        // 初始化字体渲染器
        let mut medium_renderer = TextRenderer::new(FontSize::Medium, Point::zero());
        let mut small_renderer = TextRenderer::new(FontSize::Small, Point::zero());
        let max_content_width = HITOKOTO_RECT.size.width as i32; // 格言内容最大宽度

        // 计算格言内容的行数（按单词拆分适配宽度）
        let content_lines = calculate_text_lines(&mut medium_renderer, &content, max_content_width);
        let content_line_count = content_lines.len();

        // 处理不同行数的排版逻辑
        match content_line_count {
            // 超过3行：报错，不显示作者来源，仅显示前3行
            count if count > 3 => {
                warn!(
                    "格言内容超过3行（共{}行），内容：{}，将不显示作者和来源，后续请删除该条格言",
                    count, content
                );
                // 截取前3行显示
                let truncated_content = content_lines
                    .into_iter()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ");
                draw_text_block(
                    target,
                    &mut medium_renderer,
                    &mut small_renderer,
                    &truncated_content,
                    "", // 空作者来源
                    3,  // 按3行处理
                    HITOKOTO_RECT,
                )?;
            }
            // 1-3行：正常排版
            1 | 2 | 3 => {
                draw_text_block(
                    target,
                    &mut medium_renderer,
                    &mut small_renderer,
                    &content,
                    &author_source_line,
                    content_line_count,
                    HITOKOTO_RECT,
                )?;
            }
            // 空内容：不绘制
            _ => {}
        }

        Ok(())
    }
}

/// 计算文本在指定宽度下的行数（按单词拆分，避免单词截断）
fn calculate_text_lines(renderer: &mut TextRenderer, text: &str, max_width: i32) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    // 按空格拆分单词（保留中文语义，英文按单词拆分）
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        // 纯无空格文本，按字符拆分适配宽度
        return split_non_space_text(renderer, text, max_width);
    }

    let mut lines = Vec::new();
    let mut current_line = words[0].to_string();
    let mut current_width = renderer.calculate_text_width(&current_line);

    for &word in &words[1..] {
        let word_width = renderer.calculate_text_width(word);
        let space_width = renderer.calculate_text_width(" ");
        let new_width = current_width + space_width + word_width;

        if new_width <= max_width {
            // 当前行可容纳，添加单词
            current_line.push(' ');
            current_line.push_str(word);
            current_width = new_width;
        } else {
            // 当前行不可容纳，换行
            lines.push(current_line);
            current_line = word.to_string();
            current_width = word_width;
        }
    }

    // 添加最后一行
    lines.push(current_line);
    lines
}

/// 拆分无空格文本（适配中文等无空格场景）
fn split_non_space_text(renderer: &mut TextRenderer, text: &str, max_width: i32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for c in text.chars() {
        let char_width = renderer.calculate_text_width(&c.to_string());
        if current_width + char_width <= max_width || current_line.is_empty() {
            // 当前行可容纳，添加字符
            current_line.push(c);
            current_width += char_width;
        } else {
            // 当前行不可容纳，换行
            lines.push(current_line);
            current_line = c.to_string();
            current_width = char_width;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

/// 绘制文本块（整体居中，适配不同行数排版）
fn draw_content_and_source<D>(
    target: &mut D,
    medium_renderer: &mut TextRenderer,
    small_renderer: &mut TextRenderer,
    content: &str,
    author_source: &str,
    content_line_count: usize,
    container: Rectangle,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let container_x = container.top_left.x;
    let container_y = container.top_left.y;
    let container_width = container.size.width as i32;
    let container_height = container.size.height as i32;
    let max_content_width = container_width;

    // 1. 计算内容区域尺寸
    let content_line_height = medium_renderer.get_line_height() as i32;
    let content_total_height = content_line_count as i32 * content_line_height;

    // 2. 计算作者来源区域尺寸
    let (author_source_height, author_source_width) = if author_source.is_empty() {
        (0, 0)
    } else {
        let height = small_renderer.get_line_height() as i32;
        let width = small_renderer.calculate_text_width(author_source);
        (height, width)
    };

    // 3. 计算整个文本块的总尺寸
    let total_height = content_total_height + author_source_height;
    let content_display_width = if content_line_count == 1 {
        // 单行：内容宽度为文本实际宽度
        medium_renderer.calculate_text_width(content)
    } else {
        // 多行：内容宽度为容器最大宽度
        max_content_width
    };
    // 文本块总宽度取内容和作者来源的最大值
    let total_width = content_display_width.max(author_source_width);

    // 4. 计算整体居中偏移（水平+垂直）
    let offset_x = container_x + (container_width - total_width) / 2;
    let offset_y = container_y + (container_height - total_height) / 2;

    // 5. 绘制格言内容
    medium_renderer.move_to(Point::new(offset_x, offset_y));
    if content_line_count == 1 {
        // 单行：水平居中显示
        medium_renderer.draw_text_centered(target, content, offset_x + total_width / 2)?;
    } else {
        // 2-3行：居左显示
        medium_renderer.draw_text_multiline(
            target,
            content,
            max_content_width,
            TextAlignment::Left,
        )?;
    }

    // 6. 绘制作者和来源（如果有）
    if !author_source.is_empty() {
        let author_source_y = offset_y + content_total_height;
        // 居右显示：X坐标 = 文本块左偏移 + 总宽度 - 作者来源宽度
        let author_source_x = offset_x + total_width - author_source_width;
        small_renderer.move_to(Point::new(author_source_x, author_source_y));
        small_renderer.draw_single_line(target, author_source)?;
    }

    Ok(())
}

// 兼容命名：统一对外调用接口
fn draw_text_block<D>(
    target: &mut D,
    medium_renderer: &mut TextRenderer,
    small_renderer: &mut TextRenderer,
    content: &str,
    author_source: &str,
    content_line_count: usize,
    container: Rectangle,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    draw_content_and_source(
        target,
        medium_renderer,
        small_renderer,
        content,
        author_source,
        content_line_count,
        container,
    )
}
