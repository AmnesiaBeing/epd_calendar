//! 抽象化的图像渲染器，支持多种图像绘制方式
//!
//! 本模块提供完整的图像渲染功能，包括：
//! - 基本图像绘制
//! - 在指定矩形内绘制图像
//! - 支持水平/垂直对齐
//! - 内边距配置

use embedded_graphics::geometry::Size;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use epd_waveshare::color::QuadColor;

/// 水平对齐方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorizontalAlignment {
    Left,
    Right,
    Center,
}

/// 垂直对齐方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlignment {
    Top,
    Bottom,
    Center,
}

/// 内边距配置：支持自定义上下左右边距
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Padding {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Padding {
    /// 创建统一边距配置（上下左右相同）
    pub const fn all(value: i32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// 创建自定义边距配置
    pub const fn new(top: i32, right: i32, bottom: i32, left: i32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// 图像渲染器
pub struct ImageRenderer {
    current_x: i32,
    current_y: i32,
}

impl ImageRenderer {
    /// 创建新的图像渲染器
    pub fn new(start_position: Point) -> Self {
        Self {
            current_x: start_position.x,
            current_y: start_position.y,
        }
    }

    /// 在指定位置绘制二值图像
    pub fn draw_image<D>(
        &mut self,
        display: &mut D,
        image_data: &[u8],
        size: Size,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        draw_binary_image(
            display,
            image_data,
            size,
            Point::new(self.current_x, self.current_y),
        )?;

        // 更新当前位置到图像右侧
        self.current_x += size.width as i32;

        Ok(())
    }

    /// 在指定矩形内绘制图像，支持内边距、水平/垂直对齐
    pub fn draw_in_rect<D>(
        &mut self,
        display: &mut D,
        image_data: &[u8],
        image_size: Size,
        rect: Rectangle,
        padding: Padding,
        horizontal_align: HorizontalAlignment,
        vertical_align: VerticalAlignment,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 计算内边距后的有效绘制区域
        let effective_left = rect.top_left.x + padding.left;
        let effective_top = rect.top_left.y + padding.top;
        let effective_right = rect.top_left.x + rect.size.width as i32 - padding.right;
        let effective_bottom = rect.top_left.y + rect.size.height as i32 - padding.bottom;

        // 有效区域宽度/高度
        let effective_width = (effective_right - effective_left).max(1);
        let effective_height = (effective_bottom - effective_top).max(1);

        // 计算图像位置
        let image_x = match horizontal_align {
            HorizontalAlignment::Left => effective_left,
            HorizontalAlignment::Center => {
                effective_left + (effective_width - image_size.width as i32) / 2
            }
            HorizontalAlignment::Right => effective_right - image_size.width as i32,
        };

        let image_y = match vertical_align {
            VerticalAlignment::Top => effective_top,
            VerticalAlignment::Center => {
                effective_top + (effective_height - image_size.height as i32) / 2
            }
            VerticalAlignment::Bottom => effective_bottom - image_size.height as i32,
        };

        // 绘制图像
        let saved_x = self.current_x;
        let saved_y = self.current_y;

        self.current_x = image_x;
        self.current_y = image_y;
        self.draw_image(display, image_data, image_size)?;

        // 恢复原始位置
        self.current_x = saved_x;
        self.current_y = saved_y;

        Ok(())
    }

    /// 移动到指定位置
    pub fn move_to(&mut self, position: Point) {
        self.current_x = position.x;
        self.current_y = position.y;
    }

    /// 获取当前位置
    pub fn current_position(&self) -> Point {
        Point::new(self.current_x, self.current_y)
    }
}

// 颜色定义
const BACKGROUND_COLOR: QuadColor = QuadColor::White;
const FOREGROUND_COLOR: QuadColor = QuadColor::Black;

pub fn draw_binary_image<D>(
    display: &mut D,
    image_data: &[u8],
    size: Size,
    position: Point,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = QuadColor>,
{
    let width = size.width as usize;
    let height = size.height as usize;

    // 计算每行字节数
    let bytes_per_row = (width + 7) / 8;

    // 绘制每个像素
    let pixels = (0..height).flat_map(move |y| {
        (0..width).map(move |x| {
            let byte_index = (y * bytes_per_row + x / 8) as usize;
            let bit_index = 7 - (x % 8);

            let color = if byte_index < image_data.len() {
                let byte = image_data[byte_index];
                let bit = (byte >> bit_index) & 1;
                if bit == 1 {
                    FOREGROUND_COLOR
                } else {
                    BACKGROUND_COLOR
                }
            } else {
                QuadColor::White
            };

            Pixel(Point::new(x as i32, y as i32) + position, color)
        })
    });

    display.draw_iter(pixels)
}
