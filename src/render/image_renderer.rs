//! 二值图像渲染器，把二值图像绘制到QuadColor的屏幕上
//! 注意，图像一定要在编译期生成好

use embedded_graphics::prelude::*;
use epd_waveshare::color::QuadColor;

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
