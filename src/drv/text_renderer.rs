//! 抽象化的文本渲染器，支持不同的全角、半角字体

use embedded_graphics::geometry::Size;
use embedded_graphics::prelude::*;
use epd_waveshare::color::QuadColor;

// 字体配置结构体
pub struct FontConfig {
    pub full_width_data: &'static [u8],
    pub half_width_data: &'static [u8],
    pub full_width_size: Size,
    pub half_width_size: Size,
}

// 文本渲染器
pub struct TextRenderer<C> {
    font_config: FontConfig,
    color: C,
    background_color: C,
    current_x: i32,
    current_y: i32,
    line_height: i32,
}

impl<C> TextRenderer<C>
where
    C: PixelColor + From<QuadColor> + Into<QuadColor> + Copy,
{
    pub fn new(font_config: FontConfig, color: C, background_color: C, position: Point) -> Self {
        Self {
            line_height: (font_config.full_width_size.height as i32).clone(),
            font_config,
            color,
            background_color,
            current_x: position.x,
            current_y: position.y,
        }
    }

    // 判断字符是否为半角字符
    fn is_half_width_char(c: char) -> bool {
        c.is_ascii() && !c.is_ascii_control()
    }

    // 渲染单行文本（自动处理全角半角混合）
    pub fn draw_text<D>(&mut self, display: &mut D, text: &str) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let start_x = self.current_x;

        for c in text.chars() {
            if Self::is_half_width_char(c) {
                // 半角字符
                if let Some(glyph_data) = self.get_half_width_glyph(c) {
                    draw_binary_image(
                        display,
                        glyph_data,
                        self.font_config.half_width_size,
                        Point::new(self.current_x, self.current_y),
                    )?;
                    self.current_x += self.font_config.half_width_size.width as i32;
                }
            } else {
                // 全角字符
                if let Some(glyph_data) = self.get_full_width_glyph(c) {
                    draw_binary_image(
                        display,
                        glyph_data,
                        self.font_config.full_width_size,
                        Point::new(self.current_x, self.current_y),
                    )?;
                    self.current_x += self.font_config.full_width_size.width as i32;
                }
            }
        }

        // 移动到下一行
        self.current_x = start_x;
        self.current_y += self.line_height;

        Ok(())
    }

    // 渲染右对齐文本
    pub fn draw_text_right<D>(
        &mut self,
        display: &mut D,
        text: &str,
        right_x: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let text_width = self.calculate_text_width(text) as i32;
        let start_x = right_x - text_width;

        let temp_x = self.current_x;
        self.current_x = start_x;
        let result = self.draw_text(display, text);
        self.current_x = temp_x;

        result
    }

    // 渲染居中对齐文本
    pub fn draw_text_centered<D>(
        &mut self,
        display: &mut D,
        text: &str,
        center_x: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let text_width = self.calculate_text_width(text) as i32;
        let start_x = center_x - text_width / 2;

        let temp_x = self.current_x;
        self.current_x = start_x;
        let result = self.draw_text(display, text);
        self.current_x = temp_x;

        result
    }

    // 计算文本宽度
    pub fn calculate_text_width(&self, text: &str) -> u32 {
        let mut width = 0;
        for c in text.chars() {
            if Self::is_half_width_char(c) {
                width += self.font_config.half_width_size.width;
            } else {
                width += self.font_config.full_width_size.width;
            }
        }
        width
    }

    // 移动到指定位置
    pub fn move_to(&mut self, position: Point) {
        self.current_x = position.x;
        self.current_y = position.y;
    }

    // 获取当前绘制位置
    pub fn current_position(&self) -> Point {
        Point::new(self.current_x, self.current_y)
    }

    // 设置颜色
    pub fn set_color(&mut self, color: C, background_color: C) {
        self.color = color;
        self.background_color = background_color;
    }

    // 获取全角字符的glyph数据（需要根据你的字体格式实现）
    fn get_full_width_glyph(&self, c: char) -> Option<&'static [u8]> {
        // 这里需要根据你的字体数据格式实现字符查找
        // 示例实现，需要根据实际字体格式调整
        let char_code = c as u32;
        let glyph_index = char_code as usize * self.font_config.full_width_size.width as usize;

        if glyph_index + self.font_config.full_width_size.width as usize
            <= self.font_config.full_width_data.len()
        {
            Some(&self.font_config.full_width_data[glyph_index..])
        } else {
            None
        }
    }

    // 获取半角字符的glyph数据（需要根据你的字体格式实现）
    fn get_half_width_glyph(&self, c: char) -> Option<&'static [u8]> {
        // 这里需要根据你的字体数据格式实现字符查找
        // 示例实现，需要根据实际字体格式调整
        let char_code = c as u32;
        let glyph_index = char_code as usize * self.font_config.half_width_size.width as usize;

        if glyph_index + self.font_config.half_width_size.width as usize
            <= self.font_config.half_width_data.len()
        {
            Some(&self.font_config.half_width_data[glyph_index..])
        } else {
            None
        }
    }
}

// 二值图像渲染器（你提供的）
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
                    QuadColor::Black // 这里会根据TextRenderer的颜色设置进行转换
                } else {
                    QuadColor::White
                }
            } else {
                QuadColor::White
            };

            Pixel(Point::new(x as i32, y as i32) + position, color)
        })
    });

    display.draw_iter(pixels)
}
