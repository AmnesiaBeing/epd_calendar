//! 渲染缓冲区模块
//! 提供墨水屏的帧缓冲区管理

use core::fmt::{Debug, Write};

/// 颜色枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    White,
}

impl Color {
    pub fn as_byte(self) -> u8 {
        match self {
            Color::Black => 0x00,
            Color::White => 0xFF,
        }
    }
}

/// 渲染缓冲区
#[derive(Debug, Clone)]
pub struct Framebuffer {
    width: u16,
    height: u16,
    buffer: Vec<u8>,
}

impl Framebuffer {
    /// 创建新的渲染缓冲区
    pub fn new(width: u16, height: u16) -> Self {
        let size = width as usize * height as usize;
        Self {
            width,
            height,
            buffer: vec![0xFF; size], // 默认白色
        }
    }

    /// 获取宽度
    pub fn width(&self) -> u16 {
        self.width
    }

    /// 获取高度
    pub fn height(&self) -> u16 {
        self.height
    }

    /// 获取缓冲区大小（字节）
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    /// 获取缓冲区引用
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// 获取缓冲区可变引用
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    /// 绘制像素
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: Color) -> SystemResult<()> {
        if x >= self.width || y >= self.height {
            return Err(SystemError::DisplayError);
        }

        let index = (y as usize * self.width as usize + x as usize) as usize;
        self.buffer[index] = color.as_byte();
        Ok(())
    }

    /// 绘制垂直线
    pub fn draw_vertical_line(&mut self, x: u16, y: u16, length: u16, color: Color) -> SystemResult<()> {
        for i in 0..length {
            self.draw_pixel(x, y + i, color)?;
        }
        Ok(())
    }

    /// 绘制水平线
    pub fn draw_horizontal_line(&mut self, x: u16, y: u16, length: u16, color: Color) -> SystemResult<()> {
        for i in 0..length {
            self.draw_pixel(x + i, y, color)?;
        }
        Ok(())
    }

    /// 填充矩形
    pub fn draw_rectangle(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) -> SystemResult<()> {
        for i in 0..height {
            self.draw_horizontal_line(x, y + i, width, color)?;
        }
        Ok(())
    }

    /// 清屏
    pub fn clear(&mut self, color: Color) {
        self.buffer.fill(color.as_byte());
    }

    /// 清除区域
    pub fn clear_area(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) -> SystemResult<()> {
        for i in 0..height {
            self.draw_horizontal_line(x, y + i, width, color)?;
        }
        Ok(())
    }
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self::new(800, 480) // 默认墨水屏分辨率
    }
}
