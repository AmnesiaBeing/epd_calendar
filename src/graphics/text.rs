//! 文字渲染（基于freetype-rs）

use super::buffer::{Color, FrameBuffer, WIDTH};
use freetype::face::Face;
use freetype::library::Library;
use std::path::Path;

/// 渲染单个字符
pub fn render_char(
    buffer: &mut FrameBuffer,
    face: &mut Face,
    c: char,
    x: i32,
    y: i32,
    color: Color,
    font_size: u32,
) -> Result<i32, freetype::Error> {
    // 设置字体大小
    face.set_pixel_sizes(0, font_size)?;

    // 加载字符
    let glyph_index = face.get_char_index(c as u32);
    face.load_glyph(glyph_index, freetype::face::LoadFlag::RENDER)?;

    let glyph = face.glyph();
    let bitmap = glyph.bitmap();

    // 绘制字符到缓冲区
    for (row, y_offset) in (0..bitmap.rows()).enumerate() {
        for (col, x_offset) in (0..bitmap.width()).enumerate() {
            // 获取像素灰度值
            let gray = bitmap.buffer()[(row * bitmap.width() + col) as usize];

            // 超过一半亮度的像素才绘制
            if gray > 128 {
                let draw_x = x + glyph.bitmap_left() + x_offset as i32;
                let draw_y = y + glyph.bitmap_top() - row as i32;

                // 检查坐标是否在屏幕范围内
                if draw_x >= 0
                    && draw_x < WIDTH as i32
                    && draw_y >= 0
                    && draw_y < super::buffer::HEIGHT as i32
                {
                    buffer.set_pixel(draw_x as usize, draw_y as usize, color);
                }
            }
        }
    }

    // 返回字符宽度，用于后续字符定位
    Ok((glyph.advance().x >> 6) as i32)
}

/// 绘制字符串
pub fn draw_string(
    buffer: &mut FrameBuffer,
    face: &mut Face,
    text: &str,
    x: i32,
    y: i32,
    color: Color,
    font_size: u32,
) -> Result<(), freetype::Error> {
    let mut current_x = x;

    for c in text.chars() {
        let char_width = render_char(buffer, face, c, current_x, y, color, font_size)?;
        current_x += char_width;
    }

    Ok(())
}

/// 加载字体文件
pub fn load_font<P: AsRef<Path>>(path: P) -> Result<(Library, Face), freetype::Error> {
    let library = Library::init()?;
    let face = library.new_face(path, 0)?;
    Ok((library, face))
}
