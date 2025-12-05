//! 抽象化的文本渲染器，支持不同的全角、半角字体和多行渲染
#![allow(unused)]

use alloc::string::ToString;
use alloc::vec::Vec;
use embedded_graphics::geometry::Size;
use embedded_graphics::prelude::*;
use epd_waveshare::color::QuadColor;

use crate::assets::generated_fonts::{CharWidth, FontSize};

use crate::render::draw_binary_image;

// 文本对齐方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlignment {
    Left,
    Right,
    Center,
}

// 文本渲染器
pub struct TextRenderer {
    font_size: FontSize,
    current_x: i32,
    current_y: i32,
    pub half_font_metrics: (u8, u8, u8),
    pub full_font_metrics: (u8, u8, u8),
}

impl TextRenderer {
    pub fn new(font_size: FontSize, position: Point) -> Self {
        Self {
            font_size,
            current_x: position.x,
            current_y: position.y,
            half_font_metrics: font_size.get_font_metrics(true),
            full_font_metrics: font_size.get_font_metrics(false),
        }
    }

    // 判断字符是否为半角字符
    pub fn is_half_width_char(c: char) -> bool {
        c.is_ascii() && !c.is_ascii_control()
    }

    // 渲染单行文本（自动处理全角半角混合）
    pub fn draw_text<D>(&mut self, display: &mut D, text: &str) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let start_x = self.current_x;

        for c in text.chars() {
            let position = Point::new(self.current_x, self.current_y);
            if Self::is_half_width_char(c) {
                // 半角字符
                if let Some(glyph_data) = self.font_size.get_glyph(c, CharWidth::Half) {
                    let size = Size::new(
                        self.half_font_metrics.0 as u32,
                        self.half_font_metrics.1 as u32,
                    );
                    draw_binary_image(display, glyph_data, size, position)?;
                    self.current_x += self.half_font_metrics.0 as i32;
                }
            } else {
                // 全角字符
                if let Some(glyph_data) = self.font_size.get_glyph(c, CharWidth::Full) {
                    let size = Size::new(
                        self.full_font_metrics.0 as u32,
                        self.full_font_metrics.1 as u32,
                    );
                    draw_binary_image(display, glyph_data, size, position)?;
                    self.current_x += self.full_font_metrics.0 as i32;
                }
            }
        }

        // 移动到下一行
        self.current_x = start_x;
        self.current_y += self.full_font_metrics.1 as i32;

        Ok(())
    }

    // 渲染多行文本（自动换行）
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
        let start_x = self.current_x;
        let start_y = self.current_y;

        // 将文本按单词分割（英文单词）
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.is_empty() {
            return Ok(());
        }

        // 处理第一个单词
        let mut current_line = words[0].to_string();
        let mut current_line_width = self.calculate_text_width(&current_line) as i32;

        // 处理剩余单词
        for word in &words[1..] {
            let word_width = self.calculate_text_width(word) as i32;
            let space_width = self.half_font_metrics.0 as i32; // 空格宽度

            // 检查当前行能否容纳这个单词（加上空格）
            if current_line_width + space_width + word_width <= max_width {
                // 可以容纳，添加到当前行
                current_line.push(' ');
                current_line.push_str(word);
                current_line_width += space_width + word_width;
            } else {
                // 不能容纳，绘制当前行并换行
                self.draw_line_with_alignment(
                    display,
                    &current_line,
                    start_x,
                    max_width,
                    alignment,
                )?;
                self.current_y += self.full_font_metrics.1 as i32;

                // 开始新的一行
                current_line = word.to_string();
                current_line_width = word_width;
            }
        }

        // 绘制最后一行
        if !current_line.is_empty() {
            self.draw_line_with_alignment(display, &current_line, start_x, max_width, alignment)?;
        }

        // 恢复起始位置并移动到下一行
        self.current_x = start_x;
        self.current_y = start_y;

        Ok(())
    }

    // 按指定对齐方式绘制单行
    fn draw_line_with_alignment<D>(
        &mut self,
        display: &mut D,
        text: &str,
        start_x: i32,
        max_width: i32,
        alignment: TextAlignment,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let text_width = self.calculate_text_width(text) as i32;
        let draw_x = match alignment {
            TextAlignment::Left => start_x,
            TextAlignment::Center => start_x + (max_width - text_width) / 2,
            TextAlignment::Right => start_x + max_width - text_width,
        };

        // 保存当前位置
        let saved_x = self.current_x;
        let saved_y = self.current_y;

        // 移动到计算的位置
        self.current_x = draw_x;

        // 绘制文本
        self.draw_text(display, text)?;

        // 恢复y坐标（draw_text会自动换行，我们需要撤销这个）
        self.current_y = saved_y;

        // 恢复x坐标
        self.current_x = saved_x;

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
        let text_width = self.calculate_text_width(text) as i32;
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
        let text_width = self.calculate_text_width(text) as i32;
        let start_x = center_x - text_width / 2;

        let temp_x = self.current_x;
        self.current_x = start_x;
        let result = self.draw_text(display, text);
        self.current_x = temp_x;

        result
    }

    // 计算文本宽度
    pub fn calculate_text_width(&self, text: &str) -> u32 {
        let mut width = 0;
        for c in text.chars() {
            if Self::is_half_width_char(c) {
                width += self.half_font_metrics.0 as u32;
            } else {
                width += self.full_font_metrics.0 as u32;
            }
        }
        width
    }

    // 移动到指定位置
    pub fn move_to(&mut self, position: Point) {
        self.current_x = position.x;
        self.current_y = position.y;
    }

    // 获取当前绘制位置
    pub fn current_position(&self) -> Point {
        Point::new(self.current_x, self.current_y)
    }

    // 获取文本高度（行数 * 行高）
    pub fn calculate_text_height(&self, text: &str, max_width: i32) -> u32 {
        if max_width <= 0 {
            return 0;
        }

        let words: Vec<&str> = text.split_whitespace().collect();

        if words.is_empty() {
            return 0;
        }

        let mut lines = 1;
        let mut current_line_width = self.calculate_text_width(words[0]) as i32;
        let space_width = self.half_font_metrics.0 as i32;

        for word in &words[1..] {
            let word_width = self.calculate_text_width(word) as i32;

            if current_line_width + space_width + word_width <= max_width {
                current_line_width += space_width + word_width;
            } else {
                lines += 1;
                current_line_width = word_width;
            }
        }

        (lines * self.full_font_metrics.1 as i32) as u32
    }
}
