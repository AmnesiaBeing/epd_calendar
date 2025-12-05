//! 抽象化的文本渲染器，支持基于FreeType字形参数的中英文混排、多行渲染
#![allow(unused)]

use alloc::string::ToString;
use alloc::vec::Vec;
use embedded_graphics::geometry::Size;
use embedded_graphics::prelude::*;
use epd_waveshare::color::QuadColor;

use crate::assets::generated_fonts::{FontSize, GlyphMetrics};
use crate::render::draw_binary_image;

// 文本对齐方式（保留原有）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlignment {
    Left,
    Right,
    Center,
}

// ========== 重构：文本渲染器核心逻辑 ==========
/// 文本渲染器（基于FreeType字形参数的基线排版）
pub struct TextRenderer {
    font_size: FontSize,
    current_x: i32,    // 当前绘制X坐标（基线X）
    current_y: i32,    // 当前绘制Y坐标（基线Y）
    line_height: u32,  // 动态行高（基于字体像素尺寸）
    char_spacing: i32, // 字符间距（像素），可根据需要调整
}

impl TextRenderer {
    /// 创建新的渲染器
    /// position: 基线的起始坐标（而非字符位图的起始坐标）
    pub fn new(font_size: FontSize, baseline_position: Point) -> Self {
        // 行高 = 字体像素尺寸 + 少量行间距（可自定义）
        let pixel_size = font_size.pixel_size();
        let line_height = pixel_size + 2; // 基础行高 + 2px 行间距
        let char_spacing = 1; // 字符间默认间距1px

        Self {
            font_size,
            current_x: baseline_position.x,
            current_y: baseline_position.y,
            line_height,
            char_spacing,
        }
    }

    // ========== 核心：单行文本绘制（基于基线+字符宽度步进） ==========
    /// 绘制单行文本（仅绘制，不自动换行）
    pub fn draw_single_line<D>(&mut self, display: &mut D, text: &str) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        for c in text.chars() {
            // 获取字符度量参数和位图数据
            let Some(metrics) = self.font_size.get_glyph_metrics(c) else {
                // 字符不存在，按空格宽度步进
                self.current_x += self.get_default_char_width() + self.char_spacing;
                continue;
            };
            let Some(glyph_data) = self.font_size.get_glyph_bitmap(c) else {
                self.current_x += metrics.width as i32 + self.char_spacing;
                continue;
            };

            // 计算字符位图的实际绘制坐标：
            // X = 基线X + BearingX（字符位图相对基线的水平偏移）
            // Y = 基线Y - BearingY（屏幕Y轴向下，需反向偏移以对齐基线）
            let draw_x = self.current_x + metrics.bearing_x;
            let draw_y = self.current_y - metrics.bearing_y;
            let draw_pos = Point::new(draw_x, draw_y);

            // 绘制二值位图（保留原有绘制逻辑）
            let size = Size::new(metrics.width, metrics.height);
            draw_binary_image(display, glyph_data, size, draw_pos)?;

            // 步进X坐标：字符宽度 + 字符间距（替代原Advance）
            self.current_x += metrics.width as i32 + self.char_spacing;
        }

        Ok(())
    }

    // ========== 增强：多行文本绘制（基于字符宽度的智能换行） ==========
    /// 绘制多行文本（自动换行，支持对齐）
    pub fn draw_text_multiline<D>(
        &mut self,
        display: &mut D,
        text: &str,
        max_width: i32,
        alignment: TextAlignment,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let baseline_start_x = self.current_x;
        let baseline_start_y = self.current_y;

        // 按单词分割（保留原有逻辑，适配英文换行）
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return Ok(());
        }

        // 处理第一行
        let mut current_line = words[0].to_string();
        let mut current_line_width = self.calculate_text_width(&current_line);

        // 处理剩余单词
        for word in &words[1..] {
            let word_width = self.calculate_text_width(word);
            let space_width = self.get_default_char_width() + self.char_spacing; // 空格宽度 + 间距

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
                )?;
                self.new_line(); // 移动到下一行基线

                // 新行初始化
                current_line = word.to_string();
                current_line_width = word_width;
            }
        }

        // 绘制最后一行
        if !current_line.is_empty() {
            self.draw_line_with_alignment(
                display,
                &current_line,
                baseline_start_x,
                max_width,
                alignment,
            )?;
        }

        // 恢复起始基线位置（可选：根据业务需求决定是否保留）
        self.current_x = baseline_start_x;
        self.current_y = baseline_start_y;

        Ok(())
    }

    // ========== 辅助：按对齐方式绘制单行 ==========
    fn draw_line_with_alignment<D>(
        &mut self,
        display: &mut D,
        text: &str,
        start_baseline_x: i32,
        max_width: i32,
        alignment: TextAlignment,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 计算整行的总宽度（精准宽度）
        let total_width = self.calculate_text_width(text);
        // 计算对齐后的基线起始X
        let aligned_baseline_x = match alignment {
            TextAlignment::Left => start_baseline_x,
            TextAlignment::Center => start_baseline_x + (max_width - total_width) / 2,
            TextAlignment::Right => start_baseline_x + max_width - total_width,
        };

        // 保存当前基线位置
        let saved_x = self.current_x;
        let saved_y = self.current_y;

        // 移动到对齐后的基线位置
        self.current_x = aligned_baseline_x;
        // 绘制单行
        self.draw_single_line(display, text)?;

        // 恢复基线位置
        self.current_x = saved_x;
        self.current_y = saved_y;

        Ok(())
    }

    // ========== 工具方法 ==========
    /// 计算文本的总宽度
    pub fn calculate_text_width(&self, text: &str) -> i32 {
        text.chars()
            .map(|c| {
                // 获取字符宽度，不存在则用默认宽度
                self.font_size.get_glyph_metrics(c)
                    .map(|m| m.width as i32)
                    .unwrap_or_else(|| self.get_default_char_width())
                    + self.char_spacing
            })
            .sum::<i32>()
            // 减去最后一个字符的间距（避免文本末尾多出间距）
            - if text.is_empty() { 0 } else { self.char_spacing }
    }

    /// 获取默认字符宽度（用于缺失字符）
    fn get_default_char_width(&self) -> i32 {
        // 基于字体尺寸设置默认宽度（可根据需要调整）
        match self.font_size {
            FontSize::Small => 8,
            FontSize::Medium => 12,
            FontSize::Large => 20,
        }
    }

    /// 移动到下一行基线
    pub fn new_line(&mut self) {
        self.current_x = self.current_x - self.calculate_text_width(""); // 重置X到行首（空文本宽度为0）
        self.current_y += self.line_height as i32; // 基线Y向下移动行高
    }

    /// 渲染右对齐文本（单行）
    pub fn draw_text_right<D>(
        &mut self,
        display: &mut D,
        text: &str,
        right_baseline_x: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let total_width = self.calculate_text_width(text);
        let start_x = right_baseline_x - total_width;

        let saved_x = self.current_x;
        self.current_x = start_x;
        let result = self.draw_single_line(display, text);
        self.current_x = saved_x;

        result
    }

    /// 渲染居中对齐文本（单行）
    pub fn draw_text_centered<D>(
        &mut self,
        display: &mut D,
        text: &str,
        center_baseline_x: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let total_width = self.calculate_text_width(text);
        let start_x = center_baseline_x - (total_width / 2);

        let saved_x = self.current_x;
        self.current_x = start_x;
        let result = self.draw_single_line(display, text);
        self.current_x = saved_x;

        result
    }

    /// 计算文本总高度（多行）
    pub fn calculate_text_height(&self, text: &str, max_width: i32) -> u32 {
        if max_width <= 0 || text.is_empty() {
            return 0;
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return 0;
        }

        let mut lines = 1;
        let mut current_line_width = self.calculate_text_width(words[0]);
        let space_width = self.get_default_char_width() + self.char_spacing;

        for word in &words[1..] {
            let word_width = self.calculate_text_width(word);
            if current_line_width + space_width + word_width > max_width {
                lines += 1;
                current_line_width = word_width;
            } else {
                current_line_width += space_width + word_width;
            }
        }

        lines * self.line_height
    }

    // ========== 兼容方法：保留原有draw_text（自动换行） ==========
    /// 兼容原有接口：绘制文本并自动换行（单行）
    pub fn draw_text<D>(&mut self, display: &mut D, text: &str) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        self.draw_single_line(display, text)?;
        self.new_line(); // 自动换行
        Ok(())
    }

    /// 移动到指定基线位置
    pub fn move_to(&mut self, baseline_position: Point) {
        self.current_x = baseline_position.x;
        self.current_y = baseline_position.y;
    }

    /// 获取当前基线位置
    pub fn current_baseline_position(&self) -> Point {
        Point::new(self.current_x, self.current_y)
    }

    /// 设置字符间距（可选配置）
    pub fn set_char_spacing(&mut self, spacing: i32) {
        self.char_spacing = spacing;
    }

    /// 获取当前行高
    pub fn get_line_height(&self) -> u32 {
        self.line_height
    }

    /// 设置行高（可选配置）
    pub fn set_line_height(&mut self, height: u32) {
        self.line_height = height;
    }

    /// 获取当前字体尺寸
    pub fn get_font_size(&self) -> FontSize {
        self.font_size
    }
}
