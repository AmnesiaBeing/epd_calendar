//! 图形渲染器
//! 负责绘制基本图形元素，如线条、矩形、圆形、边框等

use crate::common::error::{AppError, Result};
use crate::kernel::render::layout::nodes::{Border, Importance};
use embedded_graphics::{draw_target::DrawTarget, geometry::Point, prelude::*, primitives::*};
use epd_waveshare::color::QuadColor;

/// 图形渲染器
pub struct GraphicsRenderer;

impl GraphicsRenderer {
    /// 创建新的图形渲染器
    pub const fn new() -> Self {
        Self {}
    }

    /// 绘制边框
    pub fn draw_border<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        rect: [u16; 4],
        border: &Border,
    ) -> Result<()> {
        log::debug!("Drawing border at [{:?}] with thickness {:?}", rect, border);
        let [x, y, width, height] = rect;
        let thickness = border
            .top
            .max(border.right)
            .max(border.bottom)
            .max(border.left);

        // 绘制外边框
        Rectangle::new(
            Point::new(x as i32, y as i32),
            Size::new(width.into(), height.into()),
        )
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_color(QuadColor::Black)
                .stroke_width(thickness as u32)
                .build(),
        )
        .draw(draw_target)
        .map_err(|_| AppError::RenderFailed)?;

        Ok(())
    }

    /// 绘制线条
    pub fn draw_line<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        start: [u16; 2],
        end: [u16; 2],
        thickness: u16,
        importance: Option<Importance>,
    ) -> Result<()> {
        log::debug!(
            "Drawing line from {:?} to {:?}, thickness: {}, importance: {:?}",
            start,
            end,
            thickness,
            importance
        );
        let color = match importance {
            Some(Importance::Warning) => QuadColor::Yellow,
            Some(Importance::Critical) => QuadColor::Red,
            _ => QuadColor::Black,
        };

        Line::new(
            Point::new(start[0] as i32, start[1] as i32),
            Point::new(end[0] as i32, end[1] as i32),
        )
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_color(color)
                .stroke_width(thickness as u32)
                .build(),
        )
        .draw(draw_target)
        .map_err(|_| AppError::RenderFailed)?;

        Ok(())
    }

    /// 绘制矩形
    pub fn draw_rectangle<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        rect: [u16; 4],
        fill_importance: Option<Importance>,
        stroke_importance: Option<Importance>,
        stroke_thickness: u16,
    ) -> Result<()> {
        log::debug!(
            "Drawing rectangle at {:?}, fill: {:?}, stroke: {:?}, thickness: {}",
            rect,
            fill_importance,
            stroke_importance,
            stroke_thickness
        );
        let [x, y, width, height] = rect;
        let mut style_builder = PrimitiveStyleBuilder::new();

        // 设置填充颜色
        if let Some(importance) = fill_importance {
            let color = match importance {
                Importance::Warning => QuadColor::Yellow,
                Importance::Critical => QuadColor::Red,
                _ => QuadColor::Black,
            };
            style_builder = style_builder.fill_color(color);
        }

        // 设置描边颜色和宽度
        if let Some(importance) = stroke_importance {
            let color = match importance {
                Importance::Warning => QuadColor::Yellow,
                Importance::Critical => QuadColor::Red,
                _ => QuadColor::Black,
            };
            style_builder = style_builder
                .stroke_color(color)
                .stroke_width(stroke_thickness as u32);
        }

        Rectangle::new(
            Point::new(x as i32, y as i32),
            Size::new(width.into(), height.into()),
        )
        .into_styled(style_builder.build())
        .draw(draw_target)
        .map_err(|_| AppError::RenderFailed)?;

        Ok(())
    }

    /// 绘制圆形
    pub fn draw_circle<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        center: [u16; 2],
        radius: u16,
        fill_importance: Option<Importance>,
        stroke_importance: Option<Importance>,
        stroke_thickness: u16,
    ) -> Result<()> {
        log::debug!(
            "Drawing circle at center {:?}, radius: {}, fill: {:?}, stroke: {:?}, thickness: {}",
            center,
            radius,
            fill_importance,
            stroke_importance,
            stroke_thickness
        );
        let mut style_builder = PrimitiveStyleBuilder::new();

        // 设置填充颜色
        if let Some(importance) = fill_importance {
            let color = match importance {
                Importance::Warning => QuadColor::Yellow,
                Importance::Critical => QuadColor::Red,
                _ => QuadColor::Black,
            };
            style_builder = style_builder.fill_color(color);
        }

        // 设置描边颜色和宽度
        if let Some(importance) = stroke_importance {
            let color = match importance {
                Importance::Warning => QuadColor::Yellow,
                Importance::Critical => QuadColor::Red,
                _ => QuadColor::Black,
            };
            style_builder = style_builder
                .stroke_color(color)
                .stroke_width(stroke_thickness as u32);
        }

        Circle::new(
            Point::new(center[0] as i32, center[1] as i32),
            radius as u32,
        )
        .into_styled(style_builder.build())
        .draw(draw_target)
        .map_err(|_| AppError::RenderFailed)?;

        Ok(())
    }
}
