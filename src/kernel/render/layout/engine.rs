//! 布局渲染引擎
//! 负责协调布局的测量、计算和渲染过程

use crate::common::error::{AppError, Result};
use crate::kernel::data::DataSourceRegistry;
use crate::kernel::render::graphics::GraphicsRenderer;
use crate::kernel::render::image::ImageRenderer;
use crate::kernel::render::layout::context::RenderContext;
use crate::kernel::render::layout::evaluator::{DEFAULT_EVALUATOR, ExpressionEvaluator};
use crate::kernel::render::layout::loader::{DEFAULT_LOADER, LayoutLoader};
use crate::kernel::render::layout::nodes::*;
use crate::kernel::render::text::TextRenderer;

use embedded_graphics::draw_target::DrawTarget;
use epd_waveshare::color::QuadColor;

/// 渲染引擎
pub struct RenderEngine {
    layout_loader: LayoutLoader,
    expression_evaluator: ExpressionEvaluator,
    text_renderer: TextRenderer,
    image_renderer: ImageRenderer,
    graphics_renderer: GraphicsRenderer,
}

impl RenderEngine {
    /// 创建新的渲染引擎
    pub const fn new() -> Self {
        Self {
            layout_loader: DEFAULT_LOADER,
            expression_evaluator: DEFAULT_EVALUATOR,
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
    ) -> Result<bool> {
        log::info!("Starting layout rendering");
        // 加载布局
        let layout = self
            .layout_loader
            .load_layout()
            .map_err(|_| AppError::LayoutLoadFailed)?;
        log::debug!("Layout loaded successfully");

        // 创建渲染上下文
        let mut context = RenderContext::new(draw_target, data_source_registry);

        // 渲染根节点
        self.render_node(&layout, &mut context)?;

        Ok(context.needs_redraw)
    }

    /// 渲染节点
    fn render_node<D: DrawTarget<Color = QuadColor>>(
        &self,
        node: &LayoutNode,
        context: &mut RenderContext<'_, D>,
    ) -> Result<()> {
        // 检查节点是否应该渲染
        if !self.should_render(node, &context.data_source_registry)? {
            log::debug!("Node should not be rendered, skipping");
            return Ok(());
        }
        log::debug!("Rendering node: {} of type: {:?}", node.id(), node);

        // 根据节点类型进行渲染
        match node {
            LayoutNode::Container(container) => {
                log::debug!("Rendering container node: {}", container.id);
                let result = self.render_container(&*container, context);
                if result.is_err() {
                    log::error!("Failed to render container node: {}", container.id);
                }
                result
            }
            LayoutNode::Text(text) => {
                log::debug!("Rendering text node: {}", text.id);
                let result = self.render_text(text, context);
                if result.is_err() {
                    log::error!("Failed to render text node: {}", text.id);
                }
                result
            }
            LayoutNode::Icon(icon) => {
                log::debug!("Rendering icon node: {}", icon.id);
                let result = self.render_icon(icon, context);
                if result.is_err() {
                    log::error!("Failed to render icon node: {}", icon.id);
                }
                result
            }
            LayoutNode::Line(line) => {
                log::debug!("Rendering line node: {}", line.id);
                let result = self.render_line(line, context);
                if result.is_err() {
                    log::error!("Failed to render line node: {}", line.id);
                }
                result
            }
            LayoutNode::Rectangle(rect) => {
                log::debug!("Rendering rectangle node: {}", rect.id);
                let result = self.render_rectangle(rect, context);
                if result.is_err() {
                    log::error!("Failed to render rectangle node: {}", rect.id);
                }
                result
            }
            LayoutNode::Circle(circle) => {
                log::debug!("Rendering circle node: {}", circle.id);
                let result = self.render_circle(circle, context);
                if result.is_err() {
                    log::error!("Failed to render circle node: {}", circle.id);
                }
                result
            }
        }
    }

    /// 检查节点是否应该渲染（评估条件）
    fn should_render(&self, node: &LayoutNode, data: &DataSourceRegistry) -> Result<bool> {
        let condition = match node {
            LayoutNode::Container(container) => &container.condition,
            LayoutNode::Text(_) => &None,
            LayoutNode::Icon(_) => &None,
            LayoutNode::Line(_) => &None,
            LayoutNode::Rectangle(_) => &None,
            LayoutNode::Circle(_) => &None,
        };

        if let Some(condition) = condition {
            self.expression_evaluator
                .evaluate_condition(condition.as_str(), data)
                .map_err(|_| AppError::ConditionEvaluationFailed)
        } else {
            Ok(true)
        }
    }

    /// 渲染容器节点
    fn render_container<D: DrawTarget<Color = QuadColor>>(
        &self,
        container: &Container,
        context: &mut RenderContext<'_, D>,
    ) -> Result<()> {
        log::debug!(
            "Rendering container: {} at {:?}, border: {:?}, direction: {:?}",
            container.id,
            container.rect,
            container.border,
            container.direction
        );
        // 渲染边框
        self.graphics_renderer.draw_border(
            context.draw_target,
            container.rect,
            &container.border,
        )?;

        // 增加渲染深度
        context.push_depth();

        // 渲染子节点
        for child in &container.children {
            self.render_node(&child.node, context)?;
        }

        // 减少渲染深度
        context.pop_depth();

        Ok(())
    }

    /// 渲染文本节点
    fn render_text<D: DrawTarget<Color = QuadColor>>(
        &self,
        text: &Text,
        context: &mut RenderContext<'_, D>,
    ) -> Result<()> {
        log::debug!(
            "Rendering text node: {} with content: '{}' at {:?}",
            text.id,
            text.content,
            text.rect
        );
        // 替换占位符
        let content = self
            .expression_evaluator
            .replace_placeholders(text.content.as_str(), &context.data_source_registry)
            .map_err(|_| AppError::RenderFailed)?;
        log::debug!("Text content after placeholder replacement: '{}'", content);

        // 渲染文本
        self.text_renderer.render(
            context.draw_target,
            text.rect,
            content.as_str(),
            text.alignment,
            text.vertical_alignment,
            text.max_width,
            text.max_lines,
            text.font_size,
        )?;

        Ok(())
    }

    /// 渲染图标节点
    fn render_icon<D: DrawTarget<Color = QuadColor>>(
        &self,
        icon: &Icon,
        context: &mut RenderContext<'_, D>,
    ) -> Result<()> {
        log::debug!(
            "Rendering icon node: {} with icon_id: '{}' at {:?}",
            icon.id,
            icon.icon_id,
            icon.rect
        );
        // 替换占位符
        let icon_id = self
            .expression_evaluator
            .replace_placeholders(icon.icon_id.as_str(), &context.data_source_registry)
            .map_err(|_| AppError::RenderFailed)?;
        log::debug!("Icon ID after placeholder replacement: '{}'", icon_id);

        // 渲染图标
        self.image_renderer.render(
            context.draw_target,
            icon.rect,
            icon_id.as_str(),
            icon.importance,
        )?;

        Ok(())
    }

    /// 渲染线条节点
    fn render_line<D: DrawTarget<Color = QuadColor>>(
        &self,
        line: &Line,
        context: &mut RenderContext<'_, D>,
    ) -> Result<()> {
        self.graphics_renderer.draw_line(
            context.draw_target,
            line.start,
            line.end,
            line.thickness,
            line.importance,
        )?;

        Ok(())
    }

    /// 渲染矩形节点
    fn render_rectangle<D: DrawTarget<Color = QuadColor>>(
        &self,
        rect: &Rectangle,
        context: &mut RenderContext<'_, D>,
    ) -> Result<()> {
        self.graphics_renderer.draw_rectangle(
            context.draw_target,
            rect.rect,
            rect.fill_importance,
            rect.stroke_importance,
            rect.stroke_thickness,
        )?;

        Ok(())
    }

    /// 渲染圆形节点
    fn render_circle<D: DrawTarget<Color = QuadColor>>(
        &self,
        circle: &Circle,
        context: &mut RenderContext<'_, D>,
    ) -> Result<()> {
        self.graphics_renderer.draw_circle(
            context.draw_target,
            circle.center,
            circle.radius,
            circle.fill_importance,
            circle.stroke_importance,
            circle.stroke_thickness,
        )?;

        Ok(())
    }
}

/// 默认渲染引擎实例
pub const DEFAULT_ENGINE: RenderEngine = RenderEngine::new();
