//! 文本渲染模块
//! 
//! 目前使用简单的位图字体渲染
//! TODO: 集成 build 时生成的字体数据

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use crate::framebuffer::{Color, Framebuffer};
use lxx_calendar_common::{SystemError, SystemResult};

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
        // 简化的字符渲染 - 每个字符约 8x16 像素
        let mut cursor_x = x;
        let char_height = 16u16;
        let char_width = 8u16;

        for ch in text.chars() {
            if ch == ' ' {
                cursor_x += char_width;
                continue;
            }

            // 简单的字符渲染占位符
            // TODO: 使用实际的字体位图数据
            self.render_char_basic(framebuffer, cursor_x, y, ch)?;
            
            cursor_x += char_width + 1; // 1 像素间距
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
        // 大号字体渲染 (约 48x64 像素)
        // TODO: 使用大号字体位图
        let mut cursor_x = x;
        let char_height = 48u16;
        let char_width = 32u16;

        for ch in text.chars() {
            if ch == ' ' {
                cursor_x += char_width;
                continue;
            }

            // 简化：绘制字符边框作为占位符
            self.render_char_large(framebuffer, cursor_x, y, ch)?;
            
            cursor_x += char_width + 4;
        }

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
        // 简化的字符渲染 - 绘制字符轮廓
        // TODO: 使用实际的 8x16 字体位图
        
        // 临时实现：绘制一个小矩形表示字符位置
        framebuffer.draw_rectangle(x, y, 7, 15, Color::Black)?;
        
        Ok(())
    }

    /// 渲染单个字符 (大号)
    fn render_char_large<const SIZE: usize>(
        &self,
        framebuffer: &mut Framebuffer<SIZE>,
        x: u16,
        y: u16,
        ch: char,
    ) -> SystemResult<()> {
        // 简化的大号字符渲染
        // TODO: 使用实际的 32x48 字体位图
        
        // 临时实现：绘制一个矩形框表示字符位置
        framebuffer.draw_rectangle(x, y, 30, 46, Color::Black)?;
        
        Ok(())
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
