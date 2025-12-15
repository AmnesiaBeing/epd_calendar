//! 布局渲染引擎
//! 负责协调布局的测量、计算和渲染过程

use crate::assets::generated_layouts::get_global_layout_pool;
use crate::common::error::{AppError, Result};
use crate::kernel::data::{DataSourceRegistry, types::CacheKeyValueMap};
use crate::kernel::render::graphics::GraphicsRenderer;
use crate::kernel::render::image::ImageRenderer;
use crate::kernel::render::layout::context::RenderContext;
use crate::kernel::render::layout::nodes::*;
use crate::kernel::render::text::TextRenderer;

use alloc::vec::Vec;
use embedded_graphics::draw_target::DrawTarget;
use epd_waveshare::color::QuadColor;

/// 渲染引擎
pub struct RenderEngine {
    /// 文本渲染器
    text_renderer: TextRenderer,
    /// 图像/图标渲染器
    image_renderer: ImageRenderer,
    /// 图形渲染器（线条/矩形/圆形）
    graphics_renderer: GraphicsRenderer,
}

impl RenderEngine {
    /// 创建新的渲染引擎
    pub const fn new() -> Self {
        Self {
            text_renderer: TextRenderer::new(),
            image_renderer: ImageRenderer::new(),
            graphics_renderer: GraphicsRenderer::new(),
        }
    }

    /// 渲染布局到绘图目标
    pub fn render_layout<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        data_source_registry: &DataSourceRegistry,
        cache: &CacheKeyValueMap,
    ) -> Result<bool> {
        log::info!("Starting layout rendering");

        let layout_pool = get_global_layout_pool();

        let context = RenderContext::new(data_source_registry, cache, layout_pool);

        let root_node_id = layout_pool.root_node_id;

        let needs_redraw = self.render_node(draw_target, root_node_id, &context)?;

        Ok(needs_redraw)
    }

    /// 递归渲染单个节点（返回：是否需要重绘）
    fn render_node<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        node_id: NodeId,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        // 1. 获取节点（不存在则跳过）
        let node = match context.layout_pool.get_node(node_id) {
            Some(n) => n,
            None => {
                log::warn!("Node ID {} not found in layout pool, skipping", node_id);
                return Ok(false);
            }
        };

        // 2. 处理容器节点的条件判断
        let condition_result = match node {
            LayoutNode::Container(container) => {
                self.evaluate_node_condition(container.condition, context)
            }
            _ => Ok(true), // 非容器节点默认显示
        };

        // 3. 条件不满足则跳过渲染
        if !condition_result? {
            log::debug!("Node {} condition not met, skipping", node_id);
            return Ok(false);
        }

        // 4. 根据节点类型分发渲染，获取是否需要重绘
        let node_needs_redraw = match node {
            LayoutNode::Container(container) => {
                self.render_container(draw_target, container, node_id, context)?
            }
            LayoutNode::Text(text) => self.render_text(draw_target, text, context)?,
            LayoutNode::Icon(icon) => self.render_icon(draw_target, icon, context)?,
            LayoutNode::Line(line) => self.render_line(draw_target, line, context)?,
            LayoutNode::Rectangle(rect) => self.render_rectangle(draw_target, rect, context)?,
            LayoutNode::Circle(circle) => self.render_circle(draw_target, circle, context)?,
        };

        Ok(node_needs_redraw)
    }

    /// 渲染容器节点（递归渲染子节点，返回：是否需要重绘）
    fn render_container<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        container: &Container,
        _node_id: NodeId,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering container: {}", container.id);

        // 1. 计算容器的绝对矩形
        let container_abs_rect = context.to_absolute_rect(container.rect)?;
        let [container_x, container_y, _, _] = container_abs_rect; // 修复未使用变量警告

        // 2. 计算子节点的布局位置
        let child_positions =
            self.calculate_container_child_positions(container, container_abs_rect)?;

        // 3. 遍历子节点并渲染，汇总重绘状态
        let mut needs_redraw = false;
        for (child_idx, child_layout) in container.children.iter().enumerate() {
            // 3.1 获取子节点位置
            let child_pos = match child_positions.get(child_idx) {
                Some(p) => *p,
                None => {
                    log::warn!(
                        "Missing position for child {} of container {}",
                        child_idx,
                        container.id
                    );
                    continue;
                }
            };

            // 3.2 创建子上下文（无可变借用）
            let child_context = context.create_child_context([
                child_pos[0] as i32 - container_x as i32,
                child_pos[1] as i32 - container_y as i32,
            ]);

            // 3.3 递归渲染子节点，更新重绘状态
            let child_redraw =
                self.render_node(draw_target, child_layout.node_id, &child_context)?;
            if child_redraw {
                needs_redraw = true;
            }
        }

        Ok(needs_redraw)
    }

    /// 计算容器子节点的布局位置（水平/垂直布局 + 权重）
    fn calculate_container_child_positions(
        &self,
        container: &Container,
        container_rect: [u16; 4],
    ) -> Result<Vec<[u16; 2]>> {
        let [x, y, w, h] = container_rect;
        let mut positions = Vec::with_capacity(container.children.len());

        match container.direction {
            // 水平布局：按权重分配宽度
            ContainerDirection::Horizontal => {
                // 1. 计算总权重
                let total_weight: f32 = container
                    .children
                    .iter()
                    .map(|c| c.weight.unwrap_or(1.0))
                    .sum();

                if total_weight < MIN_WEIGHT {
                    log::error!(
                        "Container {} total weight too small: {}",
                        container.id,
                        total_weight
                    );
                    return Err(AppError::RenderError);
                }

                // 2. 计算每个子节点的宽度和起始X
                let mut current_x = x;
                for child in container.children.iter() {
                    if child.is_absolute {
                        positions.push([current_x, y]);
                        continue;
                    }

                    let child_weight = child.weight.unwrap_or(1.0);
                    let child_w = (w as f32 * (child_weight / total_weight)) as u16;
                    positions.push([current_x, y]);
                    current_x += child_w;
                    if current_x > x + w {
                        current_x = x + w;
                    }
                }
            }

            // 垂直布局：按权重分配高度
            ContainerDirection::Vertical => {
                // 1. 计算总权重
                let total_weight: f32 = container
                    .children
                    .iter()
                    .map(|c| c.weight.unwrap_or(1.0))
                    .sum();

                if total_weight < MIN_WEIGHT {
                    log::error!(
                        "Container {} total weight too small: {}",
                        container.id,
                        total_weight
                    );
                    return Err(AppError::RenderError);
                }

                // 2. 计算每个子节点的高度和起始Y
                let mut current_y = y;
                for child in container.children.iter() {
                    if child.is_absolute {
                        positions.push([x, current_y]);
                        continue;
                    }

                    let child_weight = child.weight.unwrap_or(1.0);
                    let child_h = (h as f32 * (child_weight / total_weight)) as u16;
                    positions.push([x, current_y]);
                    current_y += child_h;
                    if current_y > y + h {
                        current_y = y + h;
                    }
                }
            }
        }

        Ok(positions)
    }

    /// 渲染文本节点（返回：是否需要重绘）
    fn render_text<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        text: &Text,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering text: {}", text.id);

        // 1. 先替换占位符（无 draw_target 借用），再渲染（可变借用 draw_target）
        let content = context.replace_placeholders(text.content)?;
        let abs_rect = context.to_absolute_rect(text.rect)?;

        // 2. 渲染文本
        self.text_renderer.render(
            draw_target,
            abs_rect,
            &content,
            text.alignment,
            text.vertical_alignment,
            text.max_width,
            text.max_lines,
            text.font_size,
        )?;

        Ok(true) // 文本渲染后需要重绘
    }

    /// 渲染图标节点（返回：是否需要重绘）
    fn render_icon<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        icon: &Icon,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering icon: {}", icon.id);

        // 1. 先替换占位符，再渲染（解耦 draw_target 借用）
        let icon_id = context.replace_placeholders(icon.icon_id)?;
        let abs_rect = context.to_absolute_rect(icon.rect)?;

        // 2. 渲染图标
        self.image_renderer
            .render(draw_target, abs_rect, &icon_id, icon.importance)?;

        Ok(true) // 图标渲染后需要重绘
    }

    /// 渲染线条节点（返回：是否需要重绘）
    fn render_line<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        line: &Line,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering line: {}", line.id);

        // 1. 计算绝对起点/终点
        let abs_start = context.to_absolute_coord(line.start)?;
        let abs_end = context.to_absolute_coord(line.end)?;

        // 2. 校验线宽
        if line.thickness < MIN_THICKNESS || line.thickness > MAX_THICKNESS {
            log::error!(
                "Line {} thickness out of range: {} (must be {}~{})",
                line.id,
                line.thickness,
                MIN_THICKNESS,
                MAX_THICKNESS
            );
            return Err(AppError::RenderError);
        }

        // 3. 绘制线条
        self.graphics_renderer.draw_line(
            draw_target,
            abs_start,
            abs_end,
            line.thickness,
            line.importance,
        )?;

        Ok(true) // 线条渲染后需要重绘
    }

    /// 渲染矩形节点（返回：是否需要重绘）
    fn render_rectangle<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        rect: &Rectangle,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering rectangle: {}", rect.id);

        // 1. 计算绝对矩形
        let abs_rect = context.to_absolute_rect(rect.rect)?;

        // 2. 校验描边线宽
        if rect.stroke_thickness < MIN_THICKNESS || rect.stroke_thickness > MAX_THICKNESS {
            log::error!(
                "Rectangle {} stroke thickness out of range: {} (must be {}~{})",
                rect.id,
                rect.stroke_thickness,
                MIN_THICKNESS,
                MAX_THICKNESS
            );
            return Err(AppError::RenderError);
        }

        // 3. 绘制矩形
        self.graphics_renderer.draw_rectangle(
            draw_target,
            abs_rect,
            rect.fill_importance,
            rect.stroke_importance,
            rect.stroke_thickness,
        )?;

        Ok(true) // 矩形渲染后需要重绘
    }

    /// 渲染圆形节点（返回：是否需要重绘）
    fn render_circle<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        circle: &Circle,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering circle: {}", circle.id);

        // 1. 计算绝对圆心
        let abs_center = context.to_absolute_coord(circle.center)?;

        // 2. 校验半径
        if circle.radius > SCREEN_WIDTH / 2 || circle.radius > SCREEN_HEIGHT / 2 {
            log::error!(
                "Circle {} radius too large: {} (max {})",
                circle.id,
                circle.radius,
                SCREEN_WIDTH.min(SCREEN_HEIGHT) / 2
            );
            return Err(AppError::RenderError);
        }

        // 3. 校验描边线宽
        if circle.stroke_thickness < MIN_THICKNESS || circle.stroke_thickness > MAX_THICKNESS {
            log::error!(
                "Circle {} stroke thickness out of range: {} (must be {}~{})",
                circle.id,
                circle.stroke_thickness,
                MIN_THICKNESS,
                MAX_THICKNESS
            );
            return Err(AppError::RenderError);
        }

        // 4. 绘制圆形
        self.graphics_renderer.draw_circle(
            draw_target,
            abs_center,
            circle.radius,
            circle.fill_importance,
            circle.stroke_importance,
            circle.stroke_thickness,
        )?;

        Ok(true) // 圆形渲染后需要重绘
    }

    /// 评估节点的显示条件
    fn evaluate_node_condition(
        &self,
        condition: Option<&str>,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        match condition {
            Some(cond) => {
                log::debug!("Evaluating condition: {}", cond);
                context.evaluate_condition(cond)
            }
            None => Ok(true), // 无条件则默认显示
        }
    }
}

/// 默认渲染引擎实例
pub const DEFAULT_ENGINE: RenderEngine = RenderEngine::new();
