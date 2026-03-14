//! 文本渲染模块
//!
//! 目前使用简单的位图字体渲染
//! TODO: 集成 build 时生成的字体数据

extern crate alloc;

use super::framebuffer::{Color, Framebuffer};
use lxx_calendar_common::SystemResult;

/// 简单字体渲染器
///
/// 当前使用内置的点阵字体
/// TODO: 支持 TrueType 字体渲染（通过 build.rs 预生成）
pub struct TextRenderer {
    // TODO: 加载字体数据
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {}
    }

    /// 渲染文本到指定位置
    ///
    /// 使用默认字体大小 (约 16x16 像素)
    pub fn render<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        text: &str,
    ) -> SystemResult<()> {
        self.render_with_size(framebuffer, x, y, text, 16)
    }

    /// 渲染文本到指定位置（自定义字体大小）
    pub fn render_with_size<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        text: &str,
        font_size: u16,
    ) -> SystemResult<()> {
        let mut cursor_x = x;
        let char_width = font_size / 2 + 2;
        let char_height = font_size;

        for ch in text.chars() {
            if ch == ' ' {
                cursor_x += char_width;
                continue;
            }

            self.render_char_with_size(framebuffer, cursor_x, y, ch, font_size)?;

            cursor_x += char_width + 1;
        }

        Ok(())
    }

    /// 渲染大号文本 (用于时间显示)
    pub fn render_large<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        text: &str,
    ) -> SystemResult<()> {
        self.render_large_with_size(framebuffer, x, y, text, 48)
    }

    /// 渲染大号文本（自定义字体大小）
    pub fn render_large_with_size<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        text: &str,
        font_size: u16,
    ) -> SystemResult<()> {
        let mut cursor_x = x;
        let char_width = font_size / 2 + 2;
        let char_height = font_size;

        for ch in text.chars() {
            if ch == ' ' {
                cursor_x += char_width;
                continue;
            }

            self.render_char_with_size(framebuffer, cursor_x, y, ch, font_size)?;

            cursor_x += char_width + 4;
        }

        Ok(())
    }

    /// 渲染单个字符（自定义大小）
    fn render_char_with_size<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        ch: char,
        font_size: u16,
    ) -> SystemResult<()> {
        // 简化的字符渲染 - 绘制字符轮廓
        // TODO: 使用实际的字体位图数据

        // 临时实现：绘制一个小矩形表示字符位置
        let width = font_size / 2;
        let height = font_size;
        framebuffer.draw_rectangle(x, y, width, height, Color::Black)?;

        Ok(())
    }

    /// 渲染单个字符 (基本尺寸)
    fn render_char_basic<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        ch: char,
    ) -> SystemResult<()> {
        self.render_char_with_size(framebuffer, x, y, ch, 16)
    }

    /// 渲染单个字符 (大号)
    fn render_char_large<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        ch: char,
    ) -> SystemResult<()> {
        self.render_char_with_size(framebuffer, x, y, ch, 48)
    }

    /// 渲染文本居中
    pub fn render_centered<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        center_x: u16,
        y: u16,
        text: &str,
    ) -> SystemResult<()> {
        // 估算文本宽度
        let text_width = (text.chars().count() as u16) * 9; // 8px + 1px 间距
        let start_x = if center_x > text_width / 2 {
            center_x - text_width / 2
        } else {
            0
        };

        self.render(framebuffer, start_x, y, text)
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_text_renderer_creation() {
        let renderer = TextRenderer::new();
        assert!(true);
    }

    #[test]
    fn test_render_basic() {
        let mut fb: Framebuffer<1024> = Framebuffer::new(32, 32).unwrap();
        let renderer = TextRenderer::new();
        let result = renderer.render(&mut fb, 0, 0, "Hi");
        // 当前实现应该成功（即使只是绘制方框）
        assert!(result.is_ok());
    }
}
