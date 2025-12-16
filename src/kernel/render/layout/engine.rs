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

        // 2. 评估节点显示条件（所有节点均支持condition）
        let condition_result = self.evaluate_node_condition(node.condition(), context)?;
        if !condition_result {
            log::debug!("Node {} condition not met, skipping", node.id());
            return Ok(false);
        }

        // 3. 根据节点类型分发渲染，获取是否需要重绘
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

        // 1. 计算容器的绝对定位和实际尺寸
        let (container_abs_pos, container_size) =
            self.calculate_container_abs_layout(container, context)?;
        let [container_x, container_y] = container_abs_pos;
        let [container_w, container_h] = container_size;

        // 2. 计算子节点的布局位置（适配权重+锚点+绝对/相对布局）
        let child_layouts =
            self.calculate_container_child_layouts(container, container_abs_pos, container_size)?;

        // 3. 遍历子节点并渲染，汇总重绘状态
        let mut needs_redraw = false;
        for (child_layout, child_pos) in container.children.iter().zip(child_layouts) {
            // 跳过绝对布局子节点的相对位置计算（直接使用自身position）
            let child_abs_pos = if child_layout.is_absolute {
                context.to_absolute_pos(child_layout.node_id, context)?
            } else {
                child_pos
            };

            // 创建子上下文（基于容器坐标偏移）
            let child_context = context.create_child_context([
                child_abs_pos[0] as i32 - container_x as i32,
                child_abs_pos[1] as i32 - container_y as i32,
            ]);

            // 递归渲染子节点
            let child_redraw =
                self.render_node(draw_target, child_layout.node_id, &child_context)?;
            if child_redraw {
                needs_redraw = true;
            }
        }

        Ok(needs_redraw)
    }

    /// 计算容器自身的绝对位置和尺寸（适配新布局规则）
    fn calculate_container_abs_layout(
        &self,
        container: &Container,
        context: &RenderContext<'_>,
    ) -> Result<([u16; 2], [u16; 2])> {
        // 1. 获取容器绝对定位（position + anchor 转换）
        let abs_pos = context.calculate_absolute_position(
            container.position,
            container.anchor,
            container.width.unwrap_or(0),
            container.height.unwrap_or(0),
        )?;

        // 2. 计算容器实际尺寸（自动计算或使用指定值）
        let size = if let (Some(w), Some(h)) = (container.width, container.height) {
            [w, h]
        } else {
            self.calculate_container_auto_size(container)?
        };

        Ok((abs_pos, size))
    }

    /// 自动计算容器尺寸（基于子节点布局）
    fn calculate_container_auto_size(&self, container: &Container) -> Result<[u16; 2]> {
        let mut max_w = 0;
        let mut max_h = 0;

        match container.direction {
            Direction::Horizontal => {
                // 水平布局：宽度=最右子节点x+宽度，高度=子节点最大高度
                let mut total_width = 0;
                for child in &container.children {
                    if child.is_absolute {
                        continue; // 绝对布局子节点不参与计算
                    }

                    let child_node = context
                        .layout_pool
                        .get_node(child.node_id)
                        .ok_or(AppError::RenderError)?;
                    let child_w = child_node.width().unwrap_or(0);
                    let child_h = child_node.height().unwrap_or(0);

                    total_width += child_w;
                    if child_h > max_h {
                        max_h = child_h;
                    }
                }
                max_w = total_width;
            }
            Direction::Vertical => {
                // 垂直布局：高度=最底子节点y+高度，宽度=子节点最大宽度
                let mut total_height = 0;
                for child in &container.children {
                    if child.is_absolute {
                        continue; // 绝对布局子节点不参与计算
                    }

                    let child_node = context
                        .layout_pool
                        .get_node(child.node_id)
                        .ok_or(AppError::RenderError)?;
                    let child_w = child_node.width().unwrap_or(0);
                    let child_h = child_node.height().unwrap_or(0);

                    total_height += child_h;
                    if child_w > max_w {
                        max_w = child_w;
                    }
                }
                max_h = total_height;
            }
        }

        Ok([max_w, max_h])
    }

    /// 计算容器子节点的布局位置（适配权重+锚点+新布局规则）
    fn calculate_container_child_layouts(
        &self,
        container: &Container,
        container_pos: [u16; 2],
        container_size: [u16; 2],
    ) -> Result<Vec<[u16; 2]>> {
        let [container_x, container_y] = container_pos;
        let [container_w, container_h] = container_size;
        let mut positions = Vec::with_capacity(container.children.len());

        match container.direction {
            // 水平布局：按权重分配宽度，适配锚点对齐
            Direction::Horizontal => {
                // 1. 计算总权重（默认1.0）
                let total_weight: f32 = container
                    .children
                    .iter()
                    .filter(|c| !c.is_absolute)
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

                // 2. 计算每个子节点的位置
                let mut current_x = container_x;
                for child in container.children.iter() {
                    if child.is_absolute {
                        positions.push([current_x, container_y]);
                        continue;
                    }

                    // 获取子节点实际尺寸
                    let child_node = context
                        .layout_pool
                        .get_node(child.node_id)
                        .ok_or(AppError::RenderError)?;
                    let child_w = child_node.width().unwrap_or(0);
                    let child_h = child_node.height().unwrap_or(0);

                    // 按权重计算子节点宽度占比
                    let child_weight = child.weight.unwrap_or(1.0);
                    let allocated_w = (container_w as f32 * (child_weight / total_weight)) as u16;
                    let actual_w = if child_w > 0 { child_w } else { allocated_w };

                    // 适配垂直对齐
                    let child_y = match container.vertical_alignment {
                        VerticalAlignment::Top => container_y,
                        VerticalAlignment::Center => container_y + (container_h - child_h) / 2,
                        VerticalAlignment::Bottom => container_y + container_h - child_h,
                    };

                    positions.push([current_x, child_y]);
                    current_x += actual_w;

                    // 防止超出容器边界
                    if current_x > container_x + container_w {
                        current_x = container_x + container_w;
                    }
                }
            }

            // 垂直布局：按权重分配高度，适配锚点对齐
            Direction::Vertical => {
                // 1. 计算总权重（默认1.0）
                let total_weight: f32 = container
                    .children
                    .iter()
                    .filter(|c| !c.is_absolute)
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

                // 2. 计算每个子节点的位置
                let mut current_y = container_y;
                for child in container.children.iter() {
                    if child.is_absolute {
                        positions.push([container_x, current_y]);
                        continue;
                    }

                    // 获取子节点实际尺寸
                    let child_node = context
                        .layout_pool
                        .get_node(child.node_id)
                        .ok_or(AppError::RenderError)?;
                    let child_w = child_node.width().unwrap_or(0);
                    let child_h = child_node.height().unwrap_or(0);

                    // 按权重计算子节点高度占比
                    let child_weight = child.weight.unwrap_or(1.0);
                    let allocated_h = (container_h as f32 * (child_weight / total_weight)) as u16;
                    let actual_h = if child_h > 0 { child_h } else { allocated_h };

                    // 适配水平对齐
                    let child_x = match container.alignment {
                        TextAlignment::Left => container_x,
                        TextAlignment::Center => container_x + (container_w - child_w) / 2,
                        TextAlignment::Right => container_x + container_w - child_w,
                    };

                    positions.push([child_x, current_y]);
                    current_y += actual_h;

                    // 防止超出容器边界
                    if current_y > container_y + container_h {
                        current_y = container_y + container_h;
                    }
                }
            }
        }

        Ok(positions)
    }

    /// 渲染文本节点（适配新布局规则：position/anchor/width/height）
    fn render_text<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        text: &Text,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering text: {}", text.id);

        // 1. 替换占位符内容
        let content = context.replace_placeholders(text.content)?;

        // 2. 计算文本实际尺寸（默认值或指定值）
        let (text_w, text_h) = self.calculate_text_auto_size(text, &content)?;
        let actual_w = text.width.unwrap_or(text_w);
        let actual_h = text.height.unwrap_or(text_h);

        // 3. 计算文本绝对位置（position + anchor 转换）
        let abs_pos =
            context.calculate_absolute_position(text.position, text.anchor, actual_w, actual_h)?;

        // 4. 渲染文本（适配新尺寸和位置）
        self.text_renderer.render(
            draw_target,
            [abs_pos[0], abs_pos[1], actual_w, actual_h],
            &content,
            text.alignment,
            text.vertical_alignment,
            text.max_width,
            text.max_lines,
            text.font_size,
        )?;

        Ok(true)
    }

    /// 计算文本自动尺寸（基于字体大小和内容）
    fn calculate_text_auto_size(&self, text: &Text, content: &str) -> Result<(u16, u16)> {
        // 高度：与font_size强绑定
        let font_size_px = match text.font_size {
            FontSize::Small => 16,
            FontSize::Medium => 24,
            FontSize::Large => 40,
            FontSize::Custom(px) => px,
        };
        let height = font_size_px as u16;

        // 宽度：字符数 × (font_size × 0.6)，不超过max_width
        let char_count = content.chars().count() as f32;
        let base_width = (char_count * (font_size_px as f32 * 0.6)) as u16;
        let max_width = text.max_width.unwrap_or(SCREEN_WIDTH);
        let width = base_width.min(max_width);

        Ok((width, height))
    }

    /// 渲染图标节点（适配新布局规则：自动尺寸+position/anchor）
    fn render_icon<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        icon: &Icon,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering icon: {}", icon.id);

        // 1. 替换占位符
        let icon_id = context.replace_placeholders(icon.icon_id)?;

        // 2. 计算图标自动尺寸（从配置获取或使用默认值）
        let (icon_w, icon_h) = self.calculate_icon_auto_size(&icon_id)?;
        let actual_w = icon.width.unwrap_or(icon_w);
        let actual_h = icon.height.unwrap_or(icon_h);

        // 3. 计算图标绝对位置（position + anchor 转换）
        let abs_pos =
            context.calculate_absolute_position(icon.position, icon.anchor, actual_w, actual_h)?;

        // 4. 渲染图标
        self.image_renderer.render(
            draw_target,
            [abs_pos[0], abs_pos[1], actual_w, actual_h],
            &icon_id,
            icon.importance,
        )?;

        Ok(true)
    }

    /// 计算图标自动尺寸（匹配BuildConfig中的图标分类配置）
    fn calculate_icon_auto_size(&self, icon_id: &str) -> Result<(u16, u16)> {
        // 解析icon_id格式：{模块}:{键}
        let parts: Vec<&str> = icon_id.split(':').collect();
        if parts.len() < 2 {
            log::warn!(
                "Invalid icon_id format: {}, using default size 32x32",
                icon_id
            );
            return Ok((32, 32));
        }

        let category = parts[0];
        // 匹配本地图标分类配置（实际应从BuildConfig读取，此处简化）
        let size = match category {
            "battery" | "network" => (32, 32),
            "time_digit" => (48, 64),
            "weather" => (64, 64),
            _ => (32, 32), // 兜底默认值
        };

        Ok(size)
    }

    /// 渲染线条节点（保持原有逻辑，适配绝对坐标校验）
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
        let thickness = line.thickness.clamp(MIN_THICKNESS, MAX_THICKNESS);
        if line.thickness != thickness {
            log::warn!("Line {} thickness clamped to {}", line.id, thickness);
        }

        // 3. 绘制线条
        self.graphics_renderer.draw_line(
            draw_target,
            abs_start,
            abs_end,
            thickness,
            line.importance,
        )?;

        Ok(true)
    }

    /// 渲染矩形节点（适配新布局规则：position/anchor/width/height）
    fn render_rectangle<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        rect: &Rectangle,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering rectangle: {}", rect.id);

        // 1. 校验必填尺寸
        let width = rect.width.ok_or_else(|| {
            AppError::RenderError(format!("Rectangle {} width is required", rect.id))
        })?;
        let height = rect.height.ok_or_else(|| {
            AppError::RenderError(format!("Rectangle {} height is required", rect.id))
        })?;

        // 2. 计算矩形绝对位置（position + anchor 转换）
        let abs_pos =
            context.calculate_absolute_position(rect.position, rect.anchor, width, height)?;

        // 3. 校验描边线宽
        let stroke_thickness = rect.stroke_thickness.clamp(MIN_THICKNESS, MAX_THICKNESS);
        if rect.stroke_thickness != stroke_thickness {
            log::warn!(
                "Rectangle {} stroke thickness clamped to {}",
                rect.id,
                stroke_thickness
            );
        }

        // 4. 绘制矩形
        self.graphics_renderer.draw_rectangle(
            draw_target,
            [abs_pos[0], abs_pos[1], width, height],
            rect.fill_importance,
            rect.stroke_importance,
            stroke_thickness,
        )?;

        Ok(true)
    }

    /// 渲染圆形节点（适配新布局规则：position/anchor替代center）
    fn render_circle<D: DrawTarget<Color = QuadColor>>(
        &self,
        draw_target: &mut D,
        circle: &Circle,
        context: &RenderContext<'_>,
    ) -> Result<bool> {
        log::debug!("Rendering circle: {}", circle.id);

        // 1. 校验半径范围
        let max_radius = SCREEN_WIDTH.min(SCREEN_HEIGHT) / 2;
        let radius = circle.radius.clamp(1, max_radius);
        if circle.radius != radius {
            log::warn!("Circle {} radius clamped to {}", circle.id, radius);
        }

        // 2. 计算圆形绝对位置（position + anchor=center 转换为圆心）
        let abs_center = context.calculate_absolute_position(
            circle.position,
            circle.anchor,
            radius * 2,
            radius * 2,
        )?;
        // 转换为圆心坐标（anchor=center时，position已为圆心）
        let center = match circle.anchor {
            Anchor::Center => abs_center,
            _ => [abs_center[0] + radius, abs_center[1] + radius],
        };

        // 3. 校验描边线宽
        let stroke_thickness = circle.stroke_thickness.clamp(MIN_THICKNESS, MAX_THICKNESS);
        if circle.stroke_thickness != stroke_thickness {
            log::warn!(
                "Circle {} stroke thickness clamped to {}",
                circle.id,
                stroke_thickness
            );
        }

        // 4. 绘制圆形
        self.graphics_renderer.draw_circle(
            draw_target,
            center,
            radius,
            circle.fill_importance,
            circle.stroke_importance,
            stroke_thickness,
        )?;

        Ok(true)
    }

    /// 评估节点的显示条件（所有节点通用）
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

// ==================== 扩展 trait（用于节点属性访问） ====================
trait LayoutNodeExt {
    /// 获取节点ID
    fn id(&self) -> &str;
    /// 获取节点条件
    fn condition(&self) -> Option<&str>;
    /// 获取节点宽度
    fn width(&self) -> Option<u16>;
    /// 获取节点高度
    fn height(&self) -> Option<u16>;
}

impl LayoutNodeExt for LayoutNode {
    fn id(&self) -> &str {
        match self {
            LayoutNode::Container(c) => &c.id,
            LayoutNode::Text(t) => &t.id,
            LayoutNode::Icon(i) => &i.id,
            LayoutNode::Line(l) => &l.id,
            LayoutNode::Rectangle(r) => &r.id,
            LayoutNode::Circle(c) => &c.id,
        }
    }

    fn condition(&self) -> Option<&str> {
        match self {
            LayoutNode::Container(c) => c.condition.as_deref(),
            LayoutNode::Text(t) => t.condition.as_deref(),
            LayoutNode::Icon(i) => i.condition.as_deref(),
            LayoutNode::Line(l) => l.condition.as_deref(),
            LayoutNode::Rectangle(r) => r.condition.as_deref(),
            LayoutNode::Circle(c) => c.condition.as_deref(),
        }
    }

    fn width(&self) -> Option<u16> {
        match self {
            LayoutNode::Container(c) => c.width,
            LayoutNode::Text(t) => t.width,
            LayoutNode::Icon(i) => i.width,
            LayoutNode::Rectangle(r) => Some(r.width),
            _ => None,
        }
    }

    fn height(&self) -> Option<u16> {
        match self {
            LayoutNode::Container(c) => c.height,
            LayoutNode::Text(t) => t.height,
            LayoutNode::Icon(i) => i.height,
            LayoutNode::Rectangle(r) => Some(r.height),
            _ => None,
        }
    }
}
