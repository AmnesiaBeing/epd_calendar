//! 渲染上下文
//! 定义渲染过程中使用的上下文结构，包含渲染状态、资源引用等

use crate::common::error::Result;
use crate::kernel::data::{DataSourceRegistry, types::CacheKeyValueMap};
use crate::kernel::render::layout::nodes::*;

use embedded_graphics::draw_target::DrawTarget;
use epd_waveshare::color::QuadColor;

/// 渲染上下文
pub struct RenderContext<'a, D: DrawTarget<Color = QuadColor>> {
    /// 绘图目标
    pub draw_target: &'a mut D,
    /// 数据源注册表引用
    pub data_source_registry: &'a DataSourceRegistry,
    /// 缓存引用
    pub cache: &'a CacheKeyValueMap,
    /// 当前渲染深度（用于调试和嵌套渲染）
    pub depth: u8,
    /// 是否需要重绘
    pub needs_redraw: bool,
}

impl<'a, D: DrawTarget<Color = QuadColor>> RenderContext<'a, D> {
    /// 创建新的渲染上下文
    pub fn new(
        draw_target: &'a mut D,
        data_source_registry: &'a DataSourceRegistry,
        cache: &'a CacheKeyValueMap,
    ) -> Self {
        log::debug!("Creating new render context");
        Self {
            draw_target,
            data_source_registry,
            cache,
            depth: 0,
            needs_redraw: false,
        }
    }

    /// 增加渲染深度
    pub fn push_depth(&mut self) {
        self.depth = self.depth.saturating_add(1);
    }

    /// 减少渲染深度
    pub fn pop_depth(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// 设置重绘标志
    pub fn set_needs_redraw(&mut self) {
        self.needs_redraw = true;
    }
}

/// 布局测量结果
pub struct LayoutMeasurement {
    /// 实际宽度
    pub width: u16,
    /// 实际高度
    pub height: u16,
    /// 基线位置（相对于顶部）
    pub baseline: u16,
}

impl Default for LayoutMeasurement {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            baseline: 0,
        }
    }
}

/// 渲染器 trait
pub trait Renderer {
    /// 渲染节点
    fn render_node<D: DrawTarget<Color = QuadColor>>(
        &self,
        node: &LayoutNode,
        context: &mut RenderContext<'_, D>,
    ) -> Result<()>;

    /// 测量节点尺寸
    fn measure_node(
        &self,
        node: &LayoutNode,
        context: &RenderContext<'_, impl DrawTarget<Color = QuadColor>>,
    ) -> LayoutMeasurement;
}
