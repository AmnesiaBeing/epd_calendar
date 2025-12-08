//! 抽象化的图形渲染器，支持基本图形元素的绘制
//!
//! 本模块提供完整的图形渲染功能，包括：
//! - 矩形绘制（填充/描边）
//! - 线条绘制
//! - 圆形绘制（填充/描边）
//! - 基于重要程度的颜色映射
#![allow(unused)]

use alloc::format;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, Rectangle as GfxRectangle};
use epd_waveshare::color::QuadColor;

use crate::common::error::{AppError, Result};
use crate::kernel::render::engine::Importance;

/// 图形渲染器
pub struct GraphicsRenderer {
    // 可以根据需要添加配置参数
}

impl GraphicsRenderer {
    /// 创建新的图形渲染器
    pub fn new() -> Self {
        Self {}
    }

    /// 将重要程度转换为颜色
    pub fn importance_to_color(importance: &Importance) -> QuadColor {
        match importance {
            Importance::Normal => QuadColor::Black,   // 黑色
            Importance::Warning => QuadColor::Yellow, // 黄色
            Importance::Critical => QuadColor::Red,   // 红色
        }
    }

    /// 渲染线条元素
    pub fn draw_line<D>(
        &self,
        display: &mut D,
        start: Point,
        end: Point,
        thickness: u16,
        importance: &Importance,
    ) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let color = Self::importance_to_color(importance);

        let line_obj = Line::new(start, end).into_styled(
            embedded_graphics::primitives::PrimitiveStyle::with_stroke(color, thickness as u32),
        );

        line_obj
            .draw(display)
            .map_err(|e| AppError::RenderError(format!("线条渲染失败: {}", e)))
    }

    /// 渲染矩形元素
    pub fn draw_rectangle<D>(
        &self,
        display: &mut D,
        top_left: Point,
        size: Size,
        fill_importance: Option<&Importance>,
        stroke_importance: Option<&Importance>,
        stroke_thickness: u16,
    ) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let rectangle = GfxRectangle::new(top_left, size);

        // 设置样式
        let mut style = embedded_graphics::primitives::PrimitiveStyle::default();

        // 填充颜色
        if let Some(imp) = fill_importance {
            let color = Self::importance_to_color(imp);
            style = style.with_fill(color);
        }

        // 边框
        if stroke_thickness > 0 {
            let stroke_color = stroke_importance
                .map(Self::importance_to_color)
                .unwrap_or(Self::importance_to_color(&Importance::Normal));

            style = style.with_stroke(stroke_color, stroke_thickness as u32);
        }

        // 绘制矩形
        rectangle
            .into_styled(style)
            .draw(display)
            .map_err(|e| AppError::RenderError(format!("矩形渲染失败: {}", e)))
    }

    /// 渲染圆形元素
    pub fn draw_circle<D>(
        &self,
        display: &mut D,
        center: Point,
        diameter: u32,
        fill_importance: Option<&Importance>,
        stroke_importance: Option<&Importance>,
        stroke_thickness: u16,
    ) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let circle = Circle::new(center, diameter);

        // 设置样式
        let mut style = embedded_graphics::primitives::PrimitiveStyle::default();

        // 填充颜色
        if let Some(imp) = fill_importance {
            let color = Self::importance_to_color(imp);
            style = style.with_fill(color);
        }

        // 边框
        if stroke_thickness > 0 {
            let stroke_color = stroke_importance
                .map(Self::importance_to_color)
                .unwrap_or(Self::importance_to_color(&Importance::Normal));

            style = style.with_stroke(stroke_color, stroke_thickness as u32);
        }

        // 绘制圆形
        circle
            .into_styled(style)
            .draw(display)
            .map_err(|e| AppError::RenderError(format!("圆形渲染失败: {}", e)))
    }

    /// 渲染容器边框
    pub fn draw_border<D>(
        &self,
        display: &mut D,
        rect: GfxRectangle,
        top: u16,
        right: u16,
        bottom: u16,
        left: u16,
        importance: &Importance,
    ) -> Result<()>
    where
        D: DrawTarget<Color = QuadColor>,
    {
        let color = Self::importance_to_color(importance);
        let thickness = 1; // 边框厚度固定为1像素

        // 绘制四条边
        if top > 0 {
            let top_line = Line::new(
                Point::new(rect.top_left.x, rect.top_left.y),
                Point::new(rect.top_left.x + rect.size.width as i32, rect.top_left.y),
            )
            .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_stroke(
                color, top as u32,
            ));

            top_line.draw(display)?;
        }

        if bottom > 0 {
            let bottom_y = rect.top_left.y + rect.size.height as i32 - bottom as i32;
            let bottom_line = Line::new(
                Point::new(rect.top_left.x, bottom_y),
                Point::new(rect.top_left.x + rect.size.width as i32, bottom_y),
            )
            .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_stroke(
                color,
                bottom as u32,
            ));

            bottom_line.draw(display)?;
        }

        if left > 0 {
            let left_line = Line::new(
                Point::new(rect.top_left.x, rect.top_left.y),
                Point::new(rect.top_left.x, rect.top_left.y + rect.size.height as i32),
            )
            .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_stroke(
                color,
                left as u32,
            ));

            left_line.draw(display)?;
        }

        if right > 0 {
            let right_x = rect.top_left.x + rect.size.width as i32 - right as i32;
            let right_line = Line::new(
                Point::new(right_x, rect.top_left.y),
                Point::new(right_x, rect.top_left.y + rect.size.height as i32),
            )
            .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_stroke(
                color,
                right as u32,
            ));

            right_line.draw(display)?;
        }

        Ok(())
    }
}
