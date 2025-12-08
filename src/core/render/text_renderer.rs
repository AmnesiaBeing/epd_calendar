//! 抽象化的文本渲染器，支持基于FreeType字形参数的中英文混排、多行渲染
//!
//! 本模块提供完整的文本渲染功能，包括：
//! - 单行/多行文本渲染
//! - 自动换行和文本对齐
//! - 基于基线的精确字符定位
//! - 支持水平/垂直对齐
//! - 内边距配置
#![allow(unused)]

use alloc::string::ToString;
use alloc::vec::Vec;
use embedded_graphics::geometry::Size;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use epd_waveshare::color::QuadColor;

use crate::assets::generated_fonts::{FontSize, GlyphMetrics};
use crate::render::draw_binary_image;

/// 文本水平对齐方式
///
/// # 变体说明
/// - `Left`: 左对齐
/// - `Right`: 右对齐  
/// - `Center`: 居中对齐
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlignment {
    Left,
    Right,
    Center,
}

/// 文本垂直对齐方式
///
/// # 变体说明
/// - `Top`: 顶部对齐
/// - `Center`: 垂直居中
/// - `Bottom`: 底部对齐
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// 内边距配置：支持自定义上下左右边距
///
/// # 字段说明
/// - `top`: 上边距（像素）
/// - `right`: 右边距（像素）
/// - `bottom`: 下边距（像素）
/// - `left`: 左边距（像素）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Padding {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Padding {
    /// 创建统一边距配置（上下左右相同）
    ///
    /// # 参数
    /// - `value`: 统一的边距值（像素）
    ///
    /// # 返回值
    /// 返回配置好的Padding实例
    pub const fn all(value: i32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// 创建自定义边距配置
    ///
    /// # 参数
    /// - `top`: 上边距
    /// - `right`: 右边距
    /// - `bottom`: 下边距
    /// - `left`: 左边距
    ///
    /// # 返回值
    /// 返回配置好的Padding实例
    pub const fn new(top: i32, right: i32, bottom: i32, left: i32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// 文本渲染器（基于FreeType字形参数的基线排版）
///
/// # 字段说明
/// - `font_size`: 字体尺寸
/// - `current_x`: 当前绘制X坐标（基线X）
/// - `current_y`: 当前绘制Y坐标（基线Y）
/// - `line_height`: 动态行高（基于字体像素尺寸）
/// - `char_spacing`: 字符间距（像素）
pub struct TextRenderer {
    font_size: FontSize,
    current_x: i32,
    current_y: i32,
    line_height: u32,
    char_spacing: i32,
}

impl TextRenderer {
    /// 创建新的文本渲染器
    ///
    /// # 参数
    /// - `font_size`: 字体尺寸
    /// - `baseline_position`: 基线的起始坐标（而非字符位图的起始坐标）
    ///
    /// # 返回值
    /// 返回新的TextRenderer实例
    pub fn new(font_size: FontSize, baseline_position: Point) -> Self {
        // 行高 = 字体像素尺寸 + 少量行间距（可自定义）
        let pixel_size = font_size.pixel_size();
        let line_height = pixel_size + 2; // 基础行高 + 2px 行间距
        let char_spacing = 0; // 字符间默认间距

        Self {
            font_size,
            current_x: baseline_position.x,
            current_y: baseline_position.y,
            line_height,
            char_spacing,
        }
    }

    /// 在指定矩形内绘制文本，支持内边距、水平/垂直对齐
    ///
    /// # 参数
    /// - `display`: 显示目标
    /// - `text`: 要绘制的文本
    /// - `rect`: 目标绘制矩形
    /// - `padding`: 内边距配置
    /// - `horizontal_align`: 水平对齐方式
    /// - `vertical_align`: 垂直对齐方式
    ///
    /// # 返回值
    /// 返回绘制结果，成功为Ok(())
    ///
    /// # 错误
    /// 当绘制过程中出现错误时返回错误信息
    pub fn draw_in_rect<D>(
        &mut self,
        display: &mut D,
        text: &str,
        rect: Rectangle,
        padding: Padding,
        horizontal_align: TextAlignment,
        vertical_align: VerticalAlignment,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 空文本直接返回
        if text.is_empty() {
            return Ok(());
        }

        // ========== 步骤1：计算内边距后的有效绘制区域 ==========
        let effective_left = rect.top_left.x + padding.left as i32;
        let effective_top = rect.top_left.y + padding.top as i32;
        let effective_right = rect.top_left.x + rect.size.width as i32 - padding.right as i32;
        let effective_bottom = rect.top_left.y + rect.size.height as i32 - padding.bottom as i32;

        // 有效区域宽度/高度（确保为正）
        let effective_width = (effective_right - effective_left).max(1);
        let effective_height = (effective_bottom - effective_top).max(1);

        // ========== 步骤2：计算文本尺寸（自动换行后的总宽/总高） ==========
        let text_total_width = self.calculate_text_width(text);
        let text_total_height = self.calculate_text_height(text, effective_width) as i32;

        // ========== 步骤3：计算水平对齐后的基线起始X ==========
        let baseline_x = match horizontal_align {
            TextAlignment::Left => effective_left,
            TextAlignment::Center => effective_left + (effective_width - text_total_width) / 2,
            TextAlignment::Right => effective_right - text_total_width,
        };

        // ========== 步骤4：计算垂直对齐后的基线起始Y ==========
        let baseline_y = match vertical_align {
            VerticalAlignment::Top => {
                // 顶部对齐：基线Y = 有效区域顶部 + 行高的一半（适配基线排版）
                effective_top + (self.line_height as i32) / 2
            }
            VerticalAlignment::Center => {
                // 垂直居中：基线Y = 有效区域中心 + 行高/2 - 文本总高度/2
                effective_top
                    + (effective_height - text_total_height) / 2
                    + (self.line_height as i32) / 2
            }
            VerticalAlignment::Bottom => {
                // 底部对齐：基线Y = 有效区域底部 - 文本总高度 + 行高/2
                effective_bottom - text_total_height + (self.line_height as i32) / 2
            }
        };

        // ========== 步骤5：绘制文本（复用多行绘制逻辑） ==========
        // 保存原始基线位置（绘制后恢复）
        let saved_x = self.current_x;
        let saved_y = self.current_y;

        // 移动到计算后的基线位置
        self.move_to(Point::new(baseline_x, baseline_y));
        // 绘制多行文本（自动换行+水平对齐）
        self.draw_text_multiline(display, text, effective_width, horizontal_align)?;

        // 恢复原始基线位置
        self.move_to(Point::new(saved_x, saved_y));

        Ok(())
    }

    /// 绘制单行文本（仅绘制，不自动换行）
    ///
    /// # 参数
    /// - `display`: 显示目标
    /// - `text`: 要绘制的文本
    ///
    /// # 返回值
    /// 返回绘制结果
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

            // 步进X坐标：advance + 字符间距
            self.current_x += metrics.advance_x + self.char_spacing;
        }

        Ok(())
    }

    /// 绘制多行文本（自动换行，支持对齐）
    ///
    /// # 参数
    /// - `display`: 显示目标
    /// - `text`: 要绘制的文本
    /// - `max_width`: 最大宽度（用于自动换行）
    /// - `alignment`: 水平对齐方式
    ///
    /// # 返回值
    /// 返回绘制结果
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

    /// 按对齐方式绘制单行文本
    ///
    /// # 参数
    /// - `display`: 显示目标
    /// - `text`: 要绘制的文本
    /// - `start_baseline_x`: 起始基线X坐标
    /// - `max_width`: 最大宽度
    /// - `alignment`: 对齐方式
    ///
    /// # 返回值
    /// 返回绘制结果
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

    /// 计算文本的总宽度
    ///
    /// # 参数
    /// - `text`: 要计算宽度的文本
    ///
    /// # 返回值
    /// 返回文本的总宽度（像素）
    pub fn calculate_text_width(&self, text: &str) -> i32 {
        text.chars()
            .map(|c| {
                // 获取字符横向移动距离，不存在则用默认宽度
                self.font_size.get_glyph_metrics(c)
                    .map(|m| m.advance_x as i32)
                    .unwrap_or_else(|| self.get_default_char_width())
                    + self.char_spacing
            })
            .sum::<i32>()
            // 减去最后一个字符的间距（避免文本末尾多出间距）
            - if text.is_empty() { 0 } else { self.char_spacing }
    }

    /// 获取默认字符宽度（用于缺失字符）
    ///
    /// # 返回值
    /// 返回默认字符宽度（像素）
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
    ///
    /// # 参数
    /// - `display`: 显示目标
    /// - `text`: 要绘制的文本
    /// - `right_baseline_x`: 右对齐的基线X坐标
    ///
    /// # 返回值
    /// 返回绘制结果
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
    ///
    /// # 参数
    /// - `display`: 显示目标
    /// - `text`: 要绘制的文本
    /// - `center_baseline_x`: 居中对齐的基线X坐标
    ///
    /// # 返回值
    /// 返回绘制结果
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
    ///
    /// # 参数
    /// - `text`: 要计算高度的文本
    /// - `max_width`: 最大宽度（用于计算换行）
    ///
    /// # 返回值
    /// 返回文本的总高度（像素）
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

    /// 绘制文本并自动换行（单行）
    ///
    /// # 参数
    /// - `display`: 显示目标
    /// - `text`: 要绘制的文本
    ///
    /// # 返回值
    /// 返回绘制结果
    pub fn draw_text<D>(&mut self, display: &mut D, text: &str) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        self.draw_single_line(display, text)?;
        self.new_line(); // 自动换行
        Ok(())
    }

    /// 移动到指定基线位置
    ///
    /// # 参数
    /// - `baseline_position`: 目标基线位置
    pub fn move_to(&mut self, baseline_position: Point) {
        self.current_x = baseline_position.x;
        self.current_y = baseline_position.y;
    }

    /// 获取当前基线位置
    ///
    /// # 返回值
    /// 返回当前基线位置
    pub fn current_baseline_position(&self) -> Point {
        Point::new(self.current_x, self.current_y)
    }

    /// 设置字符间距（可选配置）
    ///
    /// # 参数
    /// - `spacing`: 字符间距（像素）
    pub fn set_char_spacing(&mut self, spacing: i32) {
        self.char_spacing = spacing;
    }

    /// 获取当前行高
    ///
    /// # 返回值
    /// 返回当前行高（像素）
    pub fn get_line_height(&self) -> u32 {
        self.line_height
    }

    /// 设置行高（可选配置）
    ///
    /// # 参数
    /// - `height`: 行高（像素）
    pub fn set_line_height(&mut self, height: u32) {
        self.line_height = height;
    }

    /// 获取当前字体尺寸
    ///
    /// # 返回值
    /// 返回当前字体尺寸
    pub fn get_font_size(&self) -> FontSize {
        self.font_size
    }
}
