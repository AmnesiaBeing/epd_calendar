//! 文本渲染模块
//! 负责将文本渲染到帧缓冲区

use super::framebuffer::Color;
use core::fmt::Write;

/// 文本渲染器
pub struct TextRenderer;

impl TextRenderer {
    pub fn new() -> Self {
        Self
    }

    /// 渲染文本
    /// 参数：
    /// - framebuffer: 渲染缓冲区
    /// - x: 起始X坐标
    /// - y: 起始Y坐标
    /// - text: 文本内容
    pub fn render(
        &self,
        framebuffer: &mut crate::renderer::Framebuffer,
        x: u16,
        y: u16,
        text: &str,
    ) -> Result<(), ()> {
        for (i, c) in text.chars().enumerate() {
            // 简单实现：每个字符占用固定宽度
            // 实际项目中应该使用字体位图
            let char_x = x + (i as u16) * 12;
            let char_y = y;

            // 绘制简单像素字符（示例：渲染数字0-9）
            self.render_char(framebuffer, char_x, char_y, c)?;
        }

        Ok(())
    }

    /// 渲染单个字符（简化版）
    /// 注意：这是示例实现，实际应该使用字体位图
    fn render_char(
        &self,
        framebuffer: &mut crate::renderer::Framebuffer,
        x: u16,
        y: u16,
        c: char,
    ) -> Result<(), ()> {
        // 简单的像素点阵字符示例
        let pixels = self.get_char_pixels(c);

        for (row, pixel_row) in pixels.iter().enumerate() {
            for (col, pixel) in pixel_row.iter().enumerate() {
                if *pixel {
                    framebuffer.draw_pixel(x + col as u16, y + row as u16, Color::White)?;
                } else {
                    framebuffer.draw_pixel(x + col as u16, y + row as u16, Color::Black)?;
                }
            }
        }

        Ok(())
    }

    /// 获取字符的像素点阵（示例数据）
    /// 8x8点阵
    fn get_char_pixels(&self, c: char) -> [[bool; 8]; 8] {
        match c {
            '0' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, true, true, false, false, true],
                    [true, false, false, true, true, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '1' => [[false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, true, true, true, true, true, true, true]],
            '2' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, true, false],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, true, false, false, false],
                    [false, false, false, true, false, false, false, false],
                    [true, true, true, true, true, true, true, true]],
            '3' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, true, false],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '4' => [[false, false, false, false, false, true, false, false],
                    [false, false, false, false, true, true, false, false],
                    [false, false, false, true, true, true, false, false],
                    [false, false, true, false, true, true, false, false],
                    [false, true, false, false, true, true, false, false],
                    [true, true, true, true, true, true, true, true],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, false, true, false, false]],
            '5' => [[true, true, true, true, true, true, true, true],
                    [true, false, false, false, false, false, false, false],
                    [true, false, false, false, false, false, false, false],
                    [true, true, true, true, true, true, true, false],
                    [false, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '6' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, false],
                    [true, false, false, false, false, false, false, false],
                    [true, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '7' => [[true, true, true, true, true, true, true, true],
                    [false, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, true, false],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, true, false, false, false],
                    [false, false, false, true, false, false, false, false],
                    [false, false, false, true, false, false, false, false],
                    [false, false, false, true, false, false, false, false]],
            '8' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '9' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, true, true, false],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            _ => [[false; 8]; 8], // 其他字符不渲染
        }
    }
}

/// 文本渲染器
pub struct TextRenderer;

impl TextRenderer {
    pub fn new() -> Self {
        Self
    }

    /// 渲染文本
    /// 参数：
    /// - framebuffer: 渲染缓冲区
    /// - x: 起始X坐标
    /// - y: 起始Y坐标
    /// - text: 文本内容
    pub fn render(
        &self,
        framebuffer: &mut crate::renderer::Framebuffer,
        x: u16,
        y: u16,
        text: &str,
    ) -> SystemResult<()> {
        for (i, c) in text.chars().enumerate() {
            // 简单实现：每个字符占用固定宽度
            // 实际项目中应该使用字体位图
            let char_x = x + (i as u16) * 12;
            let char_y = y;

            // 绘制简单像素字符（示例：渲染数字0-9）
            self.render_char(framebuffer, char_x, char_y, c)?;
        }

        Ok(())
    }

    /// 渲染单个字符（简化版）
    /// 注意：这是示例实现，实际应该使用字体位图
    fn render_char(
        &self,
        framebuffer: &mut crate::renderer::Framebuffer,
        x: u16,
        y: u16,
        c: char,
    ) -> SystemResult<()> {
        // 简单的像素点阵字符示例
        let pixels = self.get_char_pixels(c);

        for (row, pixel_row) in pixels.iter().enumerate() {
            for (col, pixel) in pixel_row.iter().enumerate() {
                if *pixel {
                    framebuffer.draw_pixel(x + col, y + row, Color::White)?;
                } else {
                    framebuffer.draw_pixel(x + col, y + row, Color::Black)?;
                }
            }
        }

        Ok(())
    }

    /// 获取字符的像素点阵（示例数据）
    /// 8x8点阵
    fn get_char_pixels(&self, c: char) -> [[bool; 8]; 8] {
        match c {
            '0' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, true, true, false, false, true],
                    [true, false, false, true, true, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '1' => [[false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, false, true, false, false, true, false, false],
                    [false, true, true, true, true, true, true, true]],
            '2' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, true, false],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, true, false, false, false],
                    [false, false, false, true, false, false, false, false],
                    [true, true, true, true, true, true, true, true]],
            '3' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, true, false],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '4' => [[false, false, false, false, false, true, false, false],
                    [false, false, false, false, true, true, false, false],
                    [false, false, false, true, true, true, false, false],
                    [false, false, true, false, true, true, false, false],
                    [false, true, false, false, true, true, false, false],
                    [true, true, true, true, true, true, true, true],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, false, true, false, false]],
            '5' => [[true, true, true, true, true, true, true, true],
                    [true, false, false, false, false, false, false, false],
                    [true, false, false, false, false, false, false, false],
                    [true, true, true, true, true, true, true, false],
                    [false, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '6' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, false],
                    [true, false, false, false, false, false, false, false],
                    [true, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '7' => [[true, true, true, true, true, true, true, true],
                    [false, false, false, false, false, false, false, true],
                    [false, false, false, false, false, false, true, false],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, true, false, false, false],
                    [false, false, false, true, false, false, false, false],
                    [false, false, false, true, false, false, false, false],
                    [false, false, false, true, false, false, false, false]],
            '8' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            '9' => [[false, true, true, true, true, true, true, false],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [true, false, false, false, false, true, true, false],
                    [false, false, false, false, false, true, false, false],
                    [false, false, false, false, false, false, false, true],
                    [true, false, false, false, false, false, false, true],
                    [false, true, true, true, true, true, true, false]],
            ':' => [[false, false],
                    [false, true],
                    [false, true],
                    [false, false],
                    [false, true],
                    [false, true],
                    [false, false],
                    [false, false]],
            _ => [[false; 8]; 8], // 其他字符不渲染
        }
    }
}
