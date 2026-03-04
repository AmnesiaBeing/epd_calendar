//! 渲染缓冲区模块
//! 提供墨水屏的帧缓冲区管理
//!
//! 支持两种模式:
//! - `no_std` 模式：使用 `heapless::Vec`，需要静态缓冲区
//! - `std` 模式：使用 `std::vec::Vec`，用于模拟器/桌面测试

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt::Debug;

/// 系统错误类型 (从 common crate 导入或定义本地版本)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FramebufferError {
    OutOfBounds,
    OutOfMemory,
    InvalidParameter,
}

pub type Result<T> = core::result::Result<T, FramebufferError>;

/// 颜色枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

    pub fn from_byte(byte: u8) -> Self {
        if byte == 0x00 {
            Color::Black
        } else {
            Color::White
        }
    }
}

/// 渲染缓冲区
/// 
/// 在嵌入式环境中，缓冲区大小需要在编译时确定。
/// 对于 800x480 的屏幕，需要 384KB 缓冲区。
/// 建议使用外部 PSRAM 或分块渲染。
pub struct Framebuffer<const SIZE: usize> {
    width: u16,
    height: u16,
    buffer: [u8; SIZE],
    used_bytes: usize,
}

impl<const SIZE: usize> Framebuffer<SIZE> {
    /// 创建新的渲染缓冲区 (初始为白色)
    pub fn new(width: u16, height: u16) -> Option<Self> {
        let required_size = (width as usize) * (height as usize);
        if required_size > SIZE {
            return None; // 缓冲区太小
        }
        
        let mut buffer = [0xFFu8; SIZE];
        // 只初始化需要使用的部分
        for i in 0..required_size {
            buffer[i] = 0xFF;
        }
        
        Some(Self {
            width,
            height,
            buffer,
            used_bytes: required_size,
        })
    }

    /// 获取宽度
    pub fn width(&self) -> u16 {
        self.width
    }

    /// 获取高度
    pub fn height(&self) -> u16 {
        self.height
    }

    /// 获取实际使用的缓冲区大小（字节）
    pub fn used_size(&self) -> usize {
        self.used_bytes
    }

    /// 获取总缓冲区大小
    pub fn total_size(&self) -> usize {
        SIZE
    }

    /// 获取缓冲区引用
    pub fn buffer(&self) -> &[u8] {
        &self.buffer[..self.used_bytes]
    }

    /// 获取缓冲区可变引用
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..self.used_bytes]
    }

    /// 获取像素索引
    #[inline]
    fn pixel_index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y as usize) * (self.width as usize) + (x as usize))
    }

    /// 绘制像素
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: Color) -> Result<()> {
        let index = self.pixel_index(x, y).ok_or(FramebufferError::OutOfBounds)?;
        self.buffer[index] = color.as_byte();
        Ok(())
    }

    /// 获取像素颜色
    pub fn get_pixel(&self, x: u16, y: u16) -> Option<Color> {
        self.pixel_index(x, y).map(|i| Color::from_byte(self.buffer[i]))
    }

    /// 绘制垂直线
    pub fn draw_vertical_line(&mut self, x: u16, y: u16, length: u16, color: Color) -> Result<()> {
        for i in 0..length {
            let ny = y.saturating_add(i);
            self.draw_pixel(x, ny, color)?;
        }
        Ok(())
    }

    /// 绘制水平线
    pub fn draw_horizontal_line(&mut self, x: u16, y: u16, length: u16, color: Color) -> Result<()> {
        for i in 0..length {
            let nx = x.saturating_add(i);
            self.draw_pixel(nx, y, color)?;
        }
        Ok(())
    }

    /// 填充矩形
    pub fn fill_rectangle(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) -> Result<()> {
        for row in 0..height {
            let ry = y.saturating_add(row);
            self.draw_horizontal_line(x, ry, width, color)?;
        }
        Ok(())
    }

    /// 绘制矩形边框
    pub fn draw_rectangle(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) -> Result<()> {
        if width < 2 || height < 2 {
            return self.fill_rectangle(x, y, width, height, color);
        }
        
        // 上下边框
        self.draw_horizontal_line(x, y, width, color)?;
        self.draw_horizontal_line(x, y + height - 1, width, color)?;
        
        // 左右边框
        for row in 1..height - 1 {
            self.draw_pixel(x, y + row, color)?;
            self.draw_pixel(x + width - 1, y + row, color)?;
        }
        
        Ok(())
    }

    /// 清屏为指定颜色
    pub fn clear(&mut self, color: Color) {
        let byte = color.as_byte();
        for i in 0..self.used_bytes {
            self.buffer[i] = byte;
        }
    }

    /// 清除指定区域
    pub fn clear_area(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) -> Result<()> {
        for row in 0..height {
            let ry = y.saturating_add(row);
            self.draw_horizontal_line(x, ry, width, color)?;
        }
        Ok(())
    }

    /// 填充整个缓冲区
    pub fn fill(&mut self, color: Color) {
        self.clear(color);
    }

    /// 复制缓冲区内容到目标数组
    pub fn copy_to(&self, dest: &mut [u8]) -> Result<()> {
        if dest.len() < self.used_bytes {
            return Err(FramebufferError::OutOfMemory);
        }
        dest[..self.used_bytes].copy_from_slice(self.buffer());
        Ok(())
    }
}

impl<const SIZE: usize> Default for Framebuffer<SIZE> {
    fn default() -> Self {
        // 默认 800x480 = 384000 字节
        Self::new(800, 480).unwrap_or(Self {
            width: 800,
            height: 480,
            buffer: [0xFFu8; SIZE],
            used_bytes: SIZE.min(800 * 480),
        })
    }
}

impl<const SIZE: usize> Debug for Framebuffer<SIZE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Framebuffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("used_bytes", &self.used_bytes)
            .field("total_size", &SIZE)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framebuffer_creation() {
        let fb: Framebuffer<1024> = Framebuffer::new(32, 32).unwrap();
        assert_eq!(fb.width(), 32);
        assert_eq!(fb.height(), 32);
        assert_eq!(fb.used_size(), 1024);
    }

    #[test]
    fn test_draw_pixel() {
        let mut fb: Framebuffer<1024> = Framebuffer::new(32, 32).unwrap();
        fb.draw_pixel(0, 0, Color::Black).unwrap();
        assert_eq!(fb.get_pixel(0, 0), Some(Color::Black));
    }

    #[test]
    fn test_out_of_bounds() {
        let mut fb: Framebuffer<1024> = Framebuffer::new(32, 32).unwrap();
        assert!(fb.draw_pixel(32, 0, Color::Black).is_err());
        assert!(fb.draw_pixel(0, 32, Color::Black).is_err());
    }
}
