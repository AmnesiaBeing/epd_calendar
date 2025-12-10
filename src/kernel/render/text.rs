//! 文本渲染器
//! 负责将文本绘制到屏幕上

use alloc::string::ToString;
use alloc::vec::Vec;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::{draw_target::DrawTarget, geometry::Size, prelude::*};
use epd_waveshare::color::QuadColor;

use crate::assets::generated_fonts::FontSize;
use crate::common::error::AppError;
use crate::common::error::Result as AppResult;
use crate::kernel::render::layout::nodes::{TextAlignment, VerticalAlignment};

/// 文本渲染器（无状态工具类）
pub struct TextRenderer;

impl TextRenderer {
    /// 创建新的文本渲染器
    pub const fn new() -> Self {
        Self {}
    }

    /// 渲染文本到指定矩形区域
    pub fn render<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        rect: [u16; 4],
        content: &str,
        alignment: TextAlignment,
        vertical_alignment: VerticalAlignment,
        max_width: Option<u16>,
        max_lines: Option<u8>,
        font_size: FontSize,
    ) -> AppResult<()> {
        if content.is_empty() {
            return Ok(());
        }

        let [x, y, width, height] = rect;
        let draw_rect = Rectangle::new(
            Point::new(x as i32, y as i32),
            Size::new(width.into(), height.into()),
        );
        let effective_width = max_width.unwrap_or(width) as i32;
        let line_height = font_size.pixel_size() + 2; // 基础行高 + 2px 行间距

        // ========== 步骤1：计算内边距后的有效绘制区域 ==========
        let effective_left = draw_rect.top_left.x;
        let effective_top = draw_rect.top_left.y;
        let effective_right = draw_rect.top_left.x + draw_rect.size.width as i32;
        let effective_bottom = draw_rect.top_left.y + draw_rect.size.height as i32;

        // 有效区域宽度/高度（确保为正）
        let actual_effective_width = (effective_right - effective_left).max(1);
        let effective_height = (effective_bottom - effective_top).max(1);
        let text_effective_width = effective_width.min(actual_effective_width);

        // ========== 步骤2：计算文本尺寸（自动换行后的总宽/总高） ==========
        let text_total_width = self.calculate_text_width(content, font_size);
        let text_total_height =
            self.calculate_text_height(content, text_effective_width, font_size) as i32;

        // ========== 步骤3：计算水平对齐后的基线起始X ==========
        let baseline_x = match alignment {
            TextAlignment::Left => effective_left,
            TextAlignment::Center => {
                effective_left + (actual_effective_width - text_total_width) / 2
            }
            TextAlignment::Right => effective_right - text_total_width,
        };

        // ========== 步骤4：计算垂直对齐后的基线起始Y ==========
        let baseline_y = match vertical_alignment {
            VerticalAlignment::Top => {
                // 顶部对齐：基线Y = 有效区域顶部 + 行高的一半（适配基线排版）
                effective_top + (line_height as i32) / 2
            }
            VerticalAlignment::Center => {
                // 垂直居中：基线Y = 有效区域中心 + 行高/2 - 文本总高度/2
                effective_top
                    + (effective_height - text_total_height) / 2
                    + (line_height as i32) / 2
            }
            VerticalAlignment::Bottom => {
                // 底部对齐：基线Y = 有效区域底部 - 文本总高度 + 行高/2
                effective_bottom - text_total_height + (line_height as i32) / 2
            }
        };

        // ========== 步骤5：绘制文本（多行绘制逻辑） ==========
        // 绘制多行文本（自动换行+水平对齐）
        self.draw_text_multiline(
            draw_target,
            content,
            text_effective_width,
            alignment,
            baseline_x,
            baseline_y,
            line_height,
            font_size,
            max_lines,
        )
    }

    /// 绘制单行文本（不考虑换行，从指定位置开始）
    fn draw_single_line<D: DrawTarget<Color = QuadColor>>(
        &self,
        display: &mut D,
        text: &str,
        start_x: i32,
        current_y: i32,
        font_size: FontSize,
    ) -> AppResult<()> {
        let mut current_x = start_x;
        for c in text.chars() {
            self.draw_char(display, c, current_x, current_y, font_size)?;
            current_x += self.get_char_width(c, font_size);
        }
        Ok(())
    }

    /// 绘制单个字符
    fn draw_char<D: DrawTarget<Color = QuadColor>>(
        &self,
        display: &mut D,
        c: char,
        x: i32,
        y: i32,
        font_size: FontSize,
    ) -> AppResult<()> {
        // 获取字符度量参数
        let Some(metrics) = font_size.get_glyph_metrics(c) else {
            // 字符不存在，按空格宽度步进
            return Ok(());
        };

        // 获取字形位图数据
        let Some(glyph_data) = font_size.get_glyph_bitmap(c) else {
            return Ok(());
        };

        // 计算字符位图的实际绘制坐标：
        // X = 基线X + BearingX（字符位图相对基线的水平偏移）
        // Y = 基线Y - BearingY（屏幕Y轴向下，需反向偏移以对齐基线）
        let draw_x = x + metrics.bearing_x;
        let draw_y = y - metrics.bearing_y;

        // 绘制字形
        self.draw_glyph(
            display,
            draw_x,
            draw_y,
            metrics.width,
            metrics.height,
            glyph_data,
        )?;

        Ok(())
    }

    /// 绘制多行文本（自动换行，支持对齐）
    fn draw_text_multiline<D: DrawTarget<Color = QuadColor>>(
        &self,
        display: &mut D,
        text: &str,
        max_width: i32,
        alignment: TextAlignment,
        baseline_start_x: i32,
        baseline_start_y: i32,
        line_height: u32,
        font_size: FontSize,
        max_lines: Option<u8>,
    ) -> AppResult<()> {
        // 按单词分割（保留原有逻辑，适配英文换行）
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return Ok(());
        }

        // 处理第一行
        let mut current_line = words[0].to_string();
        let mut current_line_width = self.calculate_text_width(&current_line, font_size);
        let mut line_count = 1i32;

        // 处理剩余单词
        for word in &words[1..] {
            if let Some(max) = max_lines {
                if line_count >= max as i32 {
                    break;
                }
            }

            let word_width = self.calculate_text_width(word, font_size);
            let space_width = self.get_default_char_width(font_size); // 空格宽度

            // 检查当前行能否容纳（单词+空格）
            if current_line_width + space_width + word_width <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
                current_line_width += space_width + word_width;
            } else {
                // 换行：绘制当前行 + 重置行状态
                self.draw_line_with_alignment(
                    display,
                    &current_line,
                    baseline_start_x,
                    max_width,
                    alignment,
                    baseline_start_y + (line_count - 1) * (line_height as i32),
                    font_size,
                )?;

                // 新行初始化
                current_line = word.to_string();
                current_line_width = word_width;
                line_count += 1;
            }
        }

        // 绘制最后一行
        if !current_line.is_empty()
            && max_lines
                .map(|max| line_count <= max as i32)
                .unwrap_or(true)
        {
            self.draw_line_with_alignment(
                display,
                &current_line,
                baseline_start_x,
                max_width,
                alignment,
                baseline_start_y + (line_count - 1) * line_height as i32,
                font_size,
            )?;
        }

        Ok(())
    }

    /// 按对齐方式绘制单行文本
    fn draw_line_with_alignment<D: DrawTarget<Color = QuadColor>>(
        &self,
        display: &mut D,
        text: &str,
        baseline_start_x: i32,
        max_width: i32,
        alignment: TextAlignment,
        current_y: i32,
        font_size: FontSize,
    ) -> AppResult<()> {
        let text_width = self.calculate_text_width(text, font_size);

        // 计算水平偏移量（基于对齐方式）
        let x_offset = match alignment {
            TextAlignment::Left => 0,
            TextAlignment::Center => (max_width - text_width) / 2,
            TextAlignment::Right => max_width - text_width,
        };

        // 绘制文本
        self.draw_single_line(
            display,
            text,
            baseline_start_x + x_offset,
            current_y,
            font_size,
        )?;

        Ok(())
    }

    /// 绘制字形
    fn draw_glyph<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        glyph_data: &[u8],
    ) -> AppResult<()> {
        // 计算每行的字节数（向上取整）
        let bytes_per_row = ((width + 7) / 8) as usize;

        for row in 0..height as usize {
            for col in 0..width as usize {
                // 计算当前像素在字节数组中的位置
                let byte_index = row * bytes_per_row + (col / 8);
                let bit_offset = 7 - (col % 8);

                // 检查是否越界
                if byte_index >= glyph_data.len() {
                    continue;
                }

                // 检查像素是否需要绘制
                if (glyph_data[byte_index] & (1 << bit_offset)) != 0 {
                    let pixel_x = x + col as i32;
                    let pixel_y = y + row as i32;

                    // 创建像素点并绘制
                    let pixel = Pixel(Point::new(pixel_x, pixel_y), QuadColor::Black);
                    if let Err(_) = draw_target.draw_iter(core::iter::once(pixel)) {
                        return Err(AppError::RenderError);
                    }
                }
            }
        }

        Ok(())
    }

    /// 计算文本宽度
    pub fn calculate_text_width(&self, text: &str, font_size: FontSize) -> i32 {
        text.chars()
            .map(|c| {
                // 获取字符横向移动距离，不存在则用默认宽度
                font_size
                    .get_glyph_metrics(c)
                    .map(|m| m.advance_x)
                    .unwrap_or_else(|| self.get_default_char_width(font_size))
            })
            .sum::<i32>()
    }

    /// 获取单个字符的宽度
    fn get_char_width(&self, c: char, font_size: FontSize) -> i32 {
        font_size
            .get_glyph_metrics(c)
            .map(|m| m.advance_x)
            .unwrap_or_else(|| self.get_default_char_width(font_size))
    }

    /// 获取默认字符宽度（用于未知字符）
    fn get_default_char_width(&self, font_size: FontSize) -> i32 {
        // 假设默认字符宽度为字体大小的60%
        (font_size.pixel_size() as i32) * 6 / 10
    }

    /// 计算文本总高度（多行）
    fn calculate_text_height(&self, text: &str, max_width: i32, font_size: FontSize) -> u32 {
        if max_width <= 0 || text.is_empty() {
            return 0;
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return 0;
        }

        let mut lines = 1;
        let mut current_line_width = self.calculate_text_width(words[0], font_size);
        let space_width = self.get_default_char_width(font_size);

        for word in &words[1..] {
            let word_width = self.calculate_text_width(word, font_size);
            if current_line_width + space_width + word_width > max_width {
                lines += 1;
                current_line_width = word_width;
            } else {
                current_line_width += space_width + word_width;
            }
        }

        lines * font_size.pixel_size()
    }
}
