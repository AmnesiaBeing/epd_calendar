use embedded_graphics::{
    draw_target::DrawTarget,
    prelude::{Pixel, Point},
};
use epd_waveshare::color::QuadColor;

use crate::kernel::render::layout::nodes::*;
use crate::{
    assets::generated_icons::IconId,
    common::error::{AppError, Result},
};

/// 图标渲染器
#[derive(Debug, Clone, Copy)]
pub struct ImageRenderer;

impl ImageRenderer {
    /// 绘制图标（遵循布局规则：anchor转换、坐标截断、尺寸验证）
    pub fn draw_icon<D>(&self, display: &mut D, icon_node: &Icon) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        // 评估icon_id表达式（实际需调用evaluator）
        let evaluated_icon_id = icon_node.evaluated_icon_id.as_str();
        let icon_id = IconId::from_str(evaluated_icon_id)?;
        let (icon_width, icon_height) = icon_id.size();

        // 计算基于anchor的最终绘制坐标
        let (x, y) = self.calculate_icon_position(icon_node, icon_width, icon_height);
        let (x_clamped, y_clamped) = Self::clamp_coords(x, y);

        // 调用核心绘制逻辑
        self.draw_icon_bitmap(display, x_clamped, y_clamped, icon_id)?;

        Ok(())
    }

    /// 基于anchor计算图标绘制坐标
    fn calculate_icon_position(
        &self,
        icon_node: &Icon,
        icon_width: u16,
        icon_height: u16,
    ) -> (i16, i16) {
        let [pos_x, pos_y] = icon_node.position;

        match icon_node.anchor {
            Anchor::TopLeft => (pos_x, pos_y),
            Anchor::Center => (
                pos_x - (icon_width / 2) as i16,
                pos_y - (icon_height / 2) as i16,
            ),
            Anchor::BottomRight => (pos_x - icon_width as i16, pos_y - icon_height as i16),
            // 其他anchor类型简化处理
            _ => (pos_x, pos_y),
        }
    }

    /// 核心图标绘制逻辑（集成提供的代码）
    fn draw_icon_bitmap<D>(
        &self,
        draw_target: &mut D,
        x: u16,
        y: u16,
        icon_id: IconId,
    ) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let icon_size = icon_id.size();
        let width = icon_size.width;
        let height = icon_size.height;
        let bitmap_data = icon_id.data();

        // 按字节绘制位图数据
        for (byte_idx, byte) in bitmap_data.iter().enumerate() {
            let y_offset = byte_idx / (width as usize / 8);
            let x_offset = (byte_idx % (width as usize / 8)) * 8;

            // 处理每个字节的8个像素
            for bit in 0..8 {
                if (byte >> (7 - bit)) & 1 != 0 {
                    let pixel_x = x + (x_offset + bit) as u16;
                    let pixel_y = y + y_offset as u16;

                    let point = Point::new(pixel_x as i32, pixel_y as i32);
                    let pixel = Pixel(point, QuadColor::Black);
                    draw_target
                        .draw_iter(core::iter::once(pixel))
                        .map_err(|_| AppError::RenderError)?;
                }
            }
        }

        Ok(())
    }

    /// 坐标截断（同graphics）
    fn clamp_coords(x: i16, y: i16) -> (u16, u16) {
        let x_clamped = x.clamp(0, 800) as u16;
        let y_clamped = y.clamp(0, 480) as u16;
        (x_clamped, y_clamped)
    }
}
