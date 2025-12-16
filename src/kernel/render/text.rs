use alloc::format;
use embedded_graphics::{
    draw_target::DrawTarget,
    prelude::{Pixel, Point},
};
use epd_waveshare::color::QuadColor;
use heapless::Vec;

use crate::kernel::{data::DynamicValue, render::layout::nodes::*};
use crate::{
    common::error::{AppError, Result},
    kernel::data::{DataSourceRegistry, types::HeaplessString},
};

/// 文本渲染器
#[derive(Debug, Clone, Copy)]
pub struct TextRenderer;

impl TextRenderer {
    /// 绘制文本（遵循布局规则：换行、max_width/max_height、变量替换、基线对齐）
    pub fn draw_text<D>(
        &self,
        display: &mut D,
        text_node: &Text,
        data: &DataSourceRegistry,
    ) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 1. 评估content表达式（替换变量）
        let evaluated_content = self.evaluate_content(text_node.content.as_str(), data)?;
        // 2. 截断超出128字符的内容（布局规则）
        let truncated_content = self.truncate_content(&evaluated_content);
        // 3. 计算文本绘制的基线坐标
        let (base_x, base_y) = self.calculate_text_baseline(text_node);
        // 4. 处理换行和绘制
        self.draw_wrapped_text(
            display,
            &truncated_content,
            base_x,
            base_y,
            text_node.font_size,
            text_node.max_width,
            text_node.max_height,
            text_node.alignment,
        )
    }

    /// 评估content中的变量引用（调用DataSourceRegistry）
    fn evaluate_content(
        &self,
        content: &str,
        data: &DataSourceRegistry,
    ) -> Result<HeaplessString<128>> {
        let mut evaluated = HeaplessString::new();
        let mut chars = content.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                // 解析变量路径
                let mut var_path = HeaplessString::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c == '}' {
                        chars.next(); // 跳过闭合花括号
                        break;
                    }
                    var_path
                        .push(next_c)
                        .map_err(|_| AppError::ContentTooLong)?;
                    chars.next();
                }

                // 从数据源获取变量值
                let var_value = data
                    .get_value_by_path_sync(&var_path)
                    .unwrap_or_else(|| DynamicValue::String(HeaplessString::new()));
                let var_str = match var_value {
                    DynamicValue::String(s) => s,
                    DynamicValue::Integer(i) => HeaplessString::from(format!("{}", i))
                        .map_err(|_| AppError::ContentTooLong)?,
                    DynamicValue::Float(f) => HeaplessString::from(format!("{:.1}", f))
                        .map_err(|_| AppError::ContentTooLong)?,
                    DynamicValue::Boolean(b) => HeaplessString::from(format!("{}", b))
                        .map_err(|_| AppError::ContentTooLong)?,
                };

                // 将变量值追加到结果
                evaluated
                    .extend(var_str.chars())
                    .map_err(|_| AppError::ContentTooLong)?;
            } else {
                // 普通字符直接追加
                evaluated.push(c).map_err(|_| AppError::ContentTooLong)?;
            }
        }

        Ok(evaluated)
    }

    /// 截断超出128字符的内容（布局规则：UTF-8完整字符截断）
    fn truncate_content(&self, content: &HeaplessString<128>) -> HeaplessString<128> {
        // HeaplessString<128> 本身已限制长度，直接返回
        content.clone()
    }

    /// 计算文本基线坐标（适配布局规则：Text无anchor，基于alignment/vertical_alignment）
    fn calculate_text_baseline(&self, text_node: &Text) -> (i32, i32) {
        let (pos_x, pos_y) = (text_node.position.0 as i32, text_node.position.1 as i32);
        let font_height = text_node.font_size.get_font_height() as i32;

        // 垂直对齐：基于文本边界框
        let base_y = match text_node.vertical_alignment {
            Alignment::Start => pos_y + font_height, // 顶部对齐：基线=顶部+字体高度
            Alignment::Center => pos_y + (font_height / 2), // 居中对齐
            Alignment::End => pos_y,                 // 底部对齐：基线=底部
        };

        (pos_x, base_y)
    }

    /// 绘制自动换行的文本
    fn draw_wrapped_text<D>(
        &self,
        display: &mut D,
        content: &HeaplessString<128>,
        base_x: i32,
        base_y: i32,
        font_size: FontSize,
        max_width: Option<u16>,
        max_height: Option<u16>,
        alignment: Alignment,
    ) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let max_width = max_width.unwrap_or(800);
        let max_height = max_height.unwrap_or(480);
        let font_height = font_size.get_font_height() as i32;
        let mut current_x = base_x;
        let mut current_y = base_y;
        let mut line_chars = Vec::<char, 64>::new(); // 每行字符缓存

        // 逐字符处理换行
        for c in content.chars() {
            if c == '\n'
                || self.would_exceed_max_width(&line_chars, c, font_size, current_x, max_width)
            {
                // 绘制当前行
                self.draw_text_line(
                    display,
                    &line_chars,
                    current_x,
                    current_y,
                    font_size,
                    alignment,
                    max_width,
                )?;

                // 重置行缓存，换行
                line_chars.clear();
                current_y += font_height + 2; // 行间距2px
                current_x = base_x;

                // 超出max_height则停止绘制
                if current_y > (base_y + max_height as i32) {
                    break;
                }

                if c != '\n' {
                    line_chars.push(c).map_err(|_| AppError::RenderError)?;
                }
            } else {
                line_chars.push(c).map_err(|_| AppError::RenderError)?;
            }
        }

        // 绘制最后一行
        if !line_chars.is_empty() {
            self.draw_text_line(
                display,
                &line_chars,
                current_x,
                current_y,
                font_size,
                alignment,
                max_width,
            )?;
        }

        Ok(())
    }

    /// 检查添加字符后是否超出max_width
    fn would_exceed_max_width(
        &self,
        line_chars: &Vec<char, 64>,
        c: char,
        font_size: FontSize,
        current_x: i32,
        max_width: u16,
    ) -> bool {
        let mut total_width = 0;
        for ch in line_chars.iter() {
            if let Some(metrics) = font_size.get_glyph_metrics(*ch) {
                total_width += metrics.advance_x;
            }
        }
        if let Some(metrics) = font_size.get_glyph_metrics(c) {
            total_width += metrics.advance_x;
        }

        (current_x + total_width) > max_width as i32
    }

    /// 绘制单行文本（处理对齐）
    fn draw_text_line<D>(
        &self,
        display: &mut D,
        chars: &Vec<char, 64>,
        base_x: i32,
        base_y: i32,
        font_size: FontSize,
        alignment: Alignment,
        max_width: u16,
    ) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 计算行宽度（用于对齐）
        let mut line_width = 0;
        for c in chars.iter() {
            if let Some(metrics) = font_size.get_glyph_metrics(*c) {
                line_width += metrics.advance_x;
            }
        }

        // 计算对齐后的起始X坐标
        let start_x = match alignment {
            Alignment::Start => base_x,
            Alignment::Center => base_x + (max_width as i32 - line_width) / 2,
            Alignment::End => base_x + (max_width as i32 - line_width),
        };

        // 逐字符绘制
        let mut current_x = start_x;
        for c in chars.iter() {
            self.draw_char(display, *c, current_x, base_y, font_size)?;
            // 步进X坐标
            if let Some(metrics) = font_size.get_glyph_metrics(*c) {
                current_x += metrics.advance_x;
            }
        }

        Ok(())
    }

    /// 绘制单个字符（集成提供的代码）
    fn draw_char<D: DrawTarget<Color = QuadColor>>(
        &self,
        display: &mut D,
        c: char,
        x: i32,
        y: i32,
        font_size: FontSize,
    ) -> Result<()> {
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

    /// 绘制字形（集成提供的代码）
    fn draw_glyph<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        glyph_data: &[u8],
    ) -> Result<()> {
        // 计算每行的字节数（向上取整）
        let bytes_per_row = width.div_ceil(8) as usize;

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

                    // 确保像素在屏幕范围内
                    if pixel_x < 0 || pixel_x > 800 || pixel_y < 0 || pixel_y > 480 {
                        continue;
                    }

                    // 创建像素点并绘制
                    let pixel = Pixel(Point::new(pixel_x, pixel_y), QuadColor::Black);
                    draw_target.draw_iter(core::iter::once(pixel))?;
                }
            }
        }

        Ok(())
    }
}
