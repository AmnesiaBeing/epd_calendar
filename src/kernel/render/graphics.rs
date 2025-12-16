use embedded_graphics::{
    draw_target::DrawTarget,
    prelude::{Pixel, Point},
};
use epd_waveshare::color::QuadColor;
use heapless::Vec;

use crate::common::error::{AppError, Result};
use crate::kernel::render::layout::nodes::*;

/// 图形渲染器
#[derive(Debug, Clone, Copy)]
pub struct GraphicsRenderer;

impl GraphicsRenderer {
    /// 绘制线段（遵循布局规则：坐标截断到[0,800]/[0,480]，thickness≥1）
    pub fn draw_line<D>(&self, display: &mut D, line: &Line) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let thickness = line.thickness.max(1); // 确保厚度≥1
        let (start_x, start_y) = Self::clamp_coords(line.start[0], line.start[1]);
        let (end_x, end_y) = Self::clamp_coords(line.end[0], line.end[1]);

        // 处理水平线/垂直线/斜线的粗细绘制
        if start_x == end_x {
            // 垂直线
            let min_y = start_y.min(end_y);
            let max_y = start_y.max(end_y);
            let half_thickness = (thickness / 2) as i32;

            for offset in -half_thickness..=half_thickness {
                let x = start_x as i32 + offset;
                if x < 0 || x > 800 {
                    continue;
                }

                for y in min_y..=max_y {
                    let pixel = Pixel(Point::new(x, y as i32), QuadColor::Black);
                    display.draw_iter(core::iter::once(pixel))?;
                }
            }
        } else if start_y == end_y {
            // 水平线
            let min_x = start_x.min(end_x);
            let max_x = start_x.max(end_x);
            let half_thickness = (thickness / 2) as i32;

            for offset in -half_thickness..=half_thickness {
                let y = start_y as i32 + offset;
                if y < 0 || y > 480 {
                    continue;
                }

                for x in min_x..=max_x {
                    let pixel = Pixel(Point::new(x as i32, y), QuadColor::Black);
                    display.draw_iter(core::iter::once(pixel))?;
                }
            }
        } else {
            // 斜线（简化处理：仅绘制中心线，厚度通过重复绘制实现）
            let mut points = Vec::<Point, 1024>::new(); // 嵌入式固定容量
            Self::bresenham_line(start_x, start_y, end_x, end_y, &mut points)?;

            let half_thickness = (thickness / 2) as i32;
            for point in points {
                for dx in -half_thickness..=half_thickness {
                    for dy in -half_thickness..=half_thickness {
                        let x = point.x + dx;
                        let y = point.y + dy;
                        if x < 0 || x > 800 || y < 0 || y > 480 {
                            continue;
                        }
                        let pixel = Pixel(Point::new(x, y), QuadColor::Black);
                        display.draw_iter(core::iter::once(pixel))?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 绘制矩形（描边，遵循布局规则：anchor转换、坐标截断、thickness≥1）
    pub fn draw_rectangle<D>(&self, display: &mut D, rect: &Rectangle) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let thickness = rect.thickness.max(1);
        let (x, y) = Self::clamp_coords(rect.position.0, rect.position.1);
        let width = rect.width.max(1);
        let height = rect.height.max(1);

        // 计算矩形四个顶点（基于anchor转换后的左上角坐标）
        let (top_left_x, top_left_y) = match rect.anchor {
            Anchor::TopLeft => (x, y),
            Anchor::Center => (x - (width / 2) as i16, y - (height / 2) as i16),
            Anchor::BottomRight => (x - width as i16, y - height as i16),
            _ => (x, y), // 其他anchor简化为TopLeft
        };

        // 绘制四条边
        let (tl_x, tl_y) = Self::clamp_coords(top_left_x, top_left_y);
        let br_x = tl_x + width as u16;
        let br_y = tl_y + height as u16;

        // 上边
        self.draw_line(
            display,
            &Line {
                id: rect.id.clone(),
                start: (tl_x as i16, tl_y as i16),
                end: (br_x as i16, tl_y as i16),
                thickness,
                layout: rect.layout,
                condition: rect.condition.clone(),
            },
        )?;

        // 下边
        self.draw_line(
            display,
            &Line {
                id: rect.id.clone(),
                start: (tl_x as i16, br_y as i16),
                end: (br_x as i16, br_y as i16),
                thickness,
                layout: rect.layout,
                condition: rect.condition.clone(),
            },
        )?;

        // 左边
        self.draw_line(
            display,
            &Line {
                id: rect.id.clone(),
                start: (tl_x as i16, tl_y as i16),
                end: (tl_x as i16, br_y as i16),
                thickness,
                layout: rect.layout,
                condition: rect.condition.clone(),
            },
        )?;

        // 右边
        self.draw_line(
            display,
            &Line {
                id: rect.id.clone(),
                start: (br_x as i16, tl_y as i16),
                end: (br_x as i16, br_y as i16),
                thickness,
                layout: rect.layout,
                condition: rect.condition.clone(),
            },
        )?;

        Ok(())
    }

    /// 坐标截断：负值→0，超出上限→800/480
    fn clamp_coords(x: i16, y: i16) -> (u16, u16) {
        let x_clamped = x.clamp(0, 800) as u16;
        let y_clamped = y.clamp(0, 480) as u16;
        (x_clamped, y_clamped)
    }

    /// 布雷森汉姆直线算法（生成斜线的像素点）
    fn bresenham_line(
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        points: &mut Vec<Point, 1024>,
    ) -> Result<()> {
        let mut x0 = x0 as i32;
        let mut y0 = y0 as i32;
        let x1 = x1 as i32;
        let y1 = y1 as i32;

        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            points
                .push(Point::new(x0, y0))
                .map_err(|_| AppError::RenderError)?;

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }

        Ok(())
    }
}
