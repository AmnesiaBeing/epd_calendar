//! 图标渲染模块
//! 基于编译期生成的图标资源，实现电子墨水屏单色图标渲染

use embedded_graphics::{draw_target::DrawTarget, prelude::*};
use epd_waveshare::color::QuadColor;

use crate::assets::generated_icons::IconId;
use crate::common::error::{AppError, Result};

/// 图标渲染器
pub struct IconRenderer;

impl IconRenderer {
    /// 渲染图标到指定位置
    pub fn render_icon<DT>(target: &mut DT, icon_id: &str, x: u32, y: u32) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: core::fmt::Debug,
    {
        // 1. 获取图标ID
        let icon = IconId::get_icon_data(icon_id)
            .ok_or_else(|| AppError::IconMissing(icon_id.to_string()))?;

        // 2. 获取图标数据和尺寸
        let icon_data = icon.data();
        let icon_size = icon.size();

        // 3. 渲染图标位图
        Self::render_icon_bitmap(target, icon_data, icon_size.width, icon_size.height, x, y)?;

        Ok(())
    }

    /// 渲染图标位图
    fn render_icon_bitmap<DT>(
        target: &mut DT,
        bitmap: &[u8],
        width: u32,
        height: u32,
        x: u32,
        y: u32,
    ) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: core::fmt::Debug,
    {
        // 遍历位图的每个像素
        for (row_idx, row) in bitmap.chunks((width + 7) as usize / 8).enumerate() {
            for (col_idx, _) in (0..width).enumerate() {
                // 计算像素在字节中的位置
                let byte_idx = col_idx / 8;
                let bit_idx = 7 - (col_idx % 8);

                // 检查像素是否为黑色
                if (row[byte_idx] >> bit_idx) & 1 == 1 {
                    let pixel_x = x + col_idx as u32;
                    let pixel_y = y + row_idx as u32;

                    // 绘制像素
                    target
                        .draw_pixel(Point::new(pixel_x as i32, pixel_y as i32), QuadColor::Black)?;
                }
            }
        }

        Ok(())
    }

    /// 渲染线条（分割线）
    pub fn render_line<DT>(target: &mut DT, x1: u32, y1: u32, x2: u32, y2: u32) -> Result<()>
    where
        DT: DrawTarget<Color = QuadColor>,
        DT::Error: core::fmt::Debug,
    {
        // 水平/垂直线优化
        if x1 == x2 {
            // 垂直线
            for y in y1..=y2 {
                target.draw_pixel(Point::new(x1 as i32, y as i32), QuadColor::Black)?;
            }
        } else if y1 == y2 {
            // 水平线
            for x in x1..=x2 {
                target.draw_pixel(Point::new(x as i32, y1 as i32), QuadColor::Black)?;
            }
        }

        Ok(())
    }
}
