//! 帧缓冲区管理（2bit/像素，支持4色）

use std::ops::{Deref, DerefMut};

/// 颜色定义（与硬件保持一致）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0b00,
    White = 0b01,
    Yellow = 0b10,
    Red = 0b11,
}

/// 屏幕分辨率
pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 480;
/// 缓冲区大小（800*480 / 4 = 96000字节）
pub const BUFFER_SIZE: usize = WIDTH * HEIGHT / 4;

/// 帧缓冲区
pub struct FrameBuffer {
    buffer: [u8; BUFFER_SIZE],
}

impl FrameBuffer {
    /// 创建新的帧缓冲区
    pub fn new() -> Self {
        Self {
            buffer: [0x55; BUFFER_SIZE], // 初始化为全白
        }
    }

    /// 设置指定位置的像素颜色
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        // 检查坐标是否有效
        if x >= WIDTH || y >= HEIGHT {
            return;
        }

        // 计算像素在缓冲区中的位置
        let index = (y * WIDTH + x) / 4;
        let shift = (3 - (x % 4)) * 2; // 每个像素占2位

        // 清除原有颜色，设置新颜色
        self.buffer[index] &= !(0x03 << shift);
        self.buffer[index] |= (color as u8) << shift;
    }

    /// 清除缓冲区（填充白色）
    pub fn clear(&mut self) {
        self.buffer.fill(0x55); // 01010101 - 全白
    }

    /// 将缓冲区数据转换为SDL可用的RGB格式（用于PC显示）
    #[cfg(feature = "pc")]
    pub fn to_rgb(&self) -> Vec<u8> {
        let mut rgb = vec![0; WIDTH * HEIGHT * 3];

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y * WIDTH + x) / 4;
                let shift = (3 - (x % 4)) * 2;
                let color_val = (self.buffer[index] >> shift) & 0x03;

                let color = match color_val {
                    0b00 => [0, 0, 0],       // 黑色
                    0b01 => [255, 255, 255], // 白色
                    0b10 => [255, 255, 0],   // 黄色
                    0b11 => [255, 0, 0],     // 红色
                    _ => unreachable!(),
                };

                let rgb_index = (y * WIDTH + x) * 3;
                rgb[rgb_index] = color[0];
                rgb[rgb_index + 1] = color[1];
                rgb[rgb_index + 2] = color[2];
            }
        }

        rgb
    }
}

impl Deref for FrameBuffer {
    type Target = [u8; BUFFER_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for FrameBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
