//! 图像/图标渲染器
//! 负责将图像和图标绘制到屏幕上

use crate::assets::generated_icons::{
    BatteryIcon, IconId, NetworkIcon, TimeDigitIcon, WeatherIcon,
};
use crate::common::error::{AppError, Result};
use crate::kernel::render::layout::nodes::Importance;
use embedded_graphics::{draw_target::DrawTarget, prelude::*};
use epd_waveshare::color::QuadColor;

/// 图像渲染器
pub struct ImageRenderer;

impl ImageRenderer {
    /// 创建新的图像渲染器
    pub const fn new() -> Self {
        Self {}
    }

    /// 渲染图标
    pub fn render<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        rect: [u16; 4],
        icon_id: &str,
        importance: Option<Importance>,
    ) -> Result<()> {
        log::debug!(
            "Rendering icon '{}' at {:?}, importance: {:?}",
            icon_id,
            rect,
            importance
        );
        // 获取图标数据
        let icon_id = IconId::get_icon_data(icon_id).ok_or(AppError::IconNotFound)?;

        // 计算图标位置（居中）
        let [x, y, width, height] = rect;
        let icon_size = icon_id.size();
        let icon_width = icon_size.width as u16;
        let icon_height = icon_size.height as u16;

        let x_pos = x + (width - icon_width) / 2;
        let y_pos = y + (height - icon_height) / 2;

        // 绘制图标
        self.draw_icon(draw_target, x_pos, y_pos, icon_id, importance)
    }

    /// 绘制图标
    fn draw_icon<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        x: u16,
        y: u16,
        icon_id: IconId,
        _importance: Option<Importance>,
    ) -> Result<()> {
        let icon_size = icon_id.size();
        let bitmap_data = icon_id.data();
        let width = icon_size.width;
        let _height = icon_size.height;

        // 按字节绘制位图数据
        for (byte_idx, byte) in bitmap_data.iter().enumerate() {
            let y_offset = byte_idx / (width as usize / 8);
            let x_offset = (byte_idx % (width as usize / 8)) * 8;

            // 处理每个字节的8个像素
            for bit in 0..8 {
                if (byte >> (7 - bit)) & 1 != 0 {
                    let pixel_x = x + (x_offset + bit) as u16;
                    let pixel_y = y + y_offset as u16;

                    // 确保像素在绘制目标范围内
                    let point = Point::new(pixel_x as i32, pixel_y as i32);
                    let pixel = Pixel(point, QuadColor::Black);
                    let _ = draw_target.draw_iter(core::iter::once(pixel));
                }
            }
        }

        Ok(())
    }
}
