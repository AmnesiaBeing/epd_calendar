//! 文本渲染模块
//! 基于编译期生成的字体资源，实现电子墨水屏单色文本渲染
//! 支持单行/多行、对齐方式、多尺寸字体

use embedded_graphics::{
    draw_target::DrawTarget, pixelcolor::BinaryColor, prelude::*, primitives::Rectangle,
};
use epd_waveshare::color::QuadColor;

use crate::assets::generated_fonts::{FontSize, GlyphMetrics, find_char_index};
use crate::common::error::{AppError, Result};

/// 文本渲染配置
pub struct TextRenderConfig {
    pub font_size: FontSize,      // 字体尺寸
    pub align: TextAlign,         // 对齐方式
    pub max_width: Option<u32>,   // 最大宽度（用于换行）
    pub max_lines: Option<usize>, // 最大行数
}

/// 文本对齐方式（与布局规则对齐）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl From<&super::layout::TextAlign> for TextAlign {
    fn from(align: &super::layout::TextAlign) -> Self {
        match align {
            super::layout::TextAlign::Left => TextAlign::Left,
            super::layout::TextAlign::Center => TextAlign::Center,
            super::layout::TextAlign::Right => TextAlign::Right,
        }
    }
}

/// 文本渲染器
pub struct TextRenderer;

impl TextRenderer {
    /// 渲染文本到指定位置
    pub fn render_text<DT>(
        target: &mut DT,
        text: &str,
        x: u32,
        y: u32,
        config: TextRenderConfig,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: core::fmt::Debug,
    {
        // 1. 转换字体尺寸
        let font_size = config.font_size;
        let line_height = font_size.pixel_size() as i32;

        // 2. 处理换行（如果指定了最大宽度）
        let lines = if let Some(max_width) = config.max_width {
            Self::wrap_text(text, font_size, max_width)?
        } else {
            vec![text.to_string()]
        };

        // 3. 限制最大行数
        let lines = if let Some(max_lines) = config.max_lines {
            lines.into_iter().take(max_lines).collect::<Vec<_>>()
        } else {
            lines
        };

        // 4. 渲染每一行文本
        for (line_idx, line) in lines.iter().enumerate() {
            let line_y = y as i32 + (line_idx as i32 * line_height);
            Self::render_single_line(target, line, x, line_y as u32, font_size, config.align)?;
        }

        Ok(())
    }

    /// 渲染单行文本（支持对齐）
    fn render_single_line<DT>(
        target: &mut DT,
        text: &str,
        x: u32,
        y: u32,
        font_size: FontSize,
        align: TextAlign,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: core::fmt::Debug,
    {
        // 1. 计算文本总宽度（用于对齐）
        let text_width = Self::calculate_text_width(text, font_size)?;

        // 2. 计算起始X坐标（根据对齐方式）
        let start_x = match align {
            TextAlign::Left => x,
            TextAlign::Center => x + (text_width / 2),
            TextAlign::Right => x + text_width,
        };

        // 3. 逐字符渲染
        let mut current_x = start_x as i32;
        let baseline_y = y as i32 + font_size.pixel_size() as i32;

        for c in text.chars() {
            // 获取字符的字形信息
            let metrics = font_size
                .get_glyph_metrics(c)
                .ok_or_else(|| AppError::FontGlyphMissing(c))?;
            let bitmap = font_size
                .get_glyph_bitmap(c)
                .ok_or_else(|| AppError::FontGlyphMissing(c))?;

            // 计算字符的绘制位置
            let glyph_x = current_x + metrics.bearing_x;
            let glyph_y = baseline_y - metrics.bearing_y;

            // 渲染字符位图
            Self::render_glyph(
                target,
                bitmap,
                metrics.width,
                metrics.height,
                glyph_x as u32,
                glyph_y as u32,
            )?;

            // 移动到下一个字符的位置
            current_x += metrics.advance_x;
        }

        Ok(())
    }

    /// 渲染单个字符的位图
    fn render_glyph<DT>(
        target: &mut DT,
        bitmap: &[u8],
        width: u32,
        height: u32,
        x: u32,
        y: u32,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: core::fmt::Debug,
    {
        // 遍历位图的每个像素
        for (row_idx, row) in bitmap.chunks((width + 7) as usize / 8).enumerate() {
            for (col_idx, _) in (0..width).enumerate() {
                // 计算像素在字节中的位置
                let byte_idx = col_idx / 8;
                let bit_idx = 7 - (col_idx % 8);

                // 检查像素是否为黑色
                if (row[byte_idx] >> bit_idx) & 1 == 1 {
                    let pixel_x = x + col_idx as u32;
                    let pixel_y = y + row_idx as u32;

                    // 绘制像素（电子墨水屏使用QuadColor::Black）
                    target
                        .draw_pixel(Point::new(pixel_x as i32, pixel_y as i32), QuadColor::Black)?;
                }
            }
        }

        Ok(())
    }

    /// 计算文本总宽度
    fn calculate_text_width(text: &str, font_size: FontSize) -> Result<u32> {
        let mut width = 0;
        for c in text.chars() {
            let metrics = font_size
                .get_glyph_metrics(c)
                .ok_or_else(|| AppError::FontGlyphMissing(c))?;
            width += metrics.advance_x as u32;
        }
        Ok(width)
    }

    /// 文本换行处理
    fn wrap_text(text: &str, font_size: FontSize, max_width: u32) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        for c in text.chars() {
            // 手动换行符
            if c == '\n' {
                lines.push(current_line);
                current_line = String::new();
                current_width = 0;
                continue;
            }

            // 获取字符宽度
            let metrics = font_size
                .get_glyph_metrics(c)
                .ok_or_else(|| AppError::FontGlyphMissing(c))?;
            let char_width = metrics.advance_x as u32;

            // 检查是否超出最大宽度
            if current_width + char_width > max_width && !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
                current_width = 0;
            }

            // 添加字符到当前行
            current_line.push(c);
            current_width += char_width;
        }

        // 添加最后一行
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        Ok(lines)
    }

    /// 自动判断文本对齐方式（单行居中，多行左对齐）
    pub fn auto_align(text: &str, font_size: FontSize, max_width: u32) -> Result<TextAlign> {
        let lines = Self::wrap_text(text, font_size, max_width)?;
        Ok(if lines.len() <= 1 {
            TextAlign::Center
        } else {
            TextAlign::Left
        })
    }
}
