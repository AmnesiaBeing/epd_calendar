//! 渲染上下文
//! 负责布局测量、坐标转换、锚点计算、渲染状态管理

use alloc::vec::Vec;

use crate::common::error::{AppError, Result};
use crate::kernel::data::{DataSourceRegistry, types::CacheKeyValueMap};
use crate::kernel::render::layout::evaluator::{DEFAULT_EVALUATOR, ExpressionEvaluator};
use crate::kernel::render::layout::nodes::*;

/// 渲染上下文（无可变状态，避免借用冲突）
pub struct RenderContext<'a> {
    // 数据源注册表（用于占位符替换/条件评估）
    pub data_source_registry: &'a DataSourceRegistry,
    // 缓存键值映射
    pub cache: &'a CacheKeyValueMap,
    // 父容器的边界（用于相对定位计算）
    parent_bounds: [u16; 4],
    // 是否需要重绘（渲染结果标记）
    pub needs_redraw: bool,
    // 布局池（扁平化存储的所有节点）
    pub layout_pool: &'a LayoutPool,
    // 累计嵌套层级（用于限制递归）
    nest_level: usize,
    // 表达式评估器（复用避免重复创建）
    evaluator: &'a ExpressionEvaluator,
}

impl<'a> RenderContext<'a> {
    /// 创建新的渲染上下文
    pub fn new(
        data_source_registry: &'a DataSourceRegistry,
        cache: &'a CacheKeyValueMap,
        layout_pool: &'a LayoutPool,
    ) -> Self {
        // 根容器边界默认是屏幕尺寸
        let parent_bounds = [0, 0, SCREEN_WIDTH, SCREEN_HEIGHT];

        Self {
            data_source_registry,
            cache,
            layout_pool,
            parent_bounds,
            needs_redraw: false,
            nest_level: 0,
            evaluator: &DEFAULT_EVALUATOR,
        }
    }

    /// 基于锚点计算元素的绝对坐标
    /// - position: 元素的锚点坐标 [x, y]
    /// - anchor: 锚点类型（如TopLeft/Center）
    /// - size: 元素尺寸 [width, height]
    /// - parent_bounds: 父容器边界 [x, y, width, height]
    /// - is_absolute: 是否绝对定位（绝对定位则基于屏幕，否则基于父容器）
    pub fn calculate_absolute_position(
        &self,
        position: [u16; 2],
        anchor: Anchor,
        size: [u16; 2],
        is_absolute: bool,
    ) -> [u16; 2] {
        let (base_x, base_y) = if is_absolute {
            // 绝对定位：基于屏幕左上角
            (0, 0)
        } else {
            // 相对定位：基于父容器左上角
            (self.parent_bounds[0], self.parent_bounds[1])
        };

        let (width, height) = (size[0], size[1]);
        let (pos_x, pos_y) = (position[0], position[1]);

        // 根据锚点调整最终坐标
        let abs_x = match anchor {
            Anchor::TopLeft | Anchor::CenterLeft | Anchor::BottomLeft => base_x + pos_x,
            Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => base_x + pos_x - width / 2,
            Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => base_x + pos_x - width,
        };

        let abs_y = match anchor {
            Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => base_y + pos_y,
            Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => {
                base_y + pos_y - height / 2
            }
            Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => {
                base_y + pos_y - height
            }
        };

        // 确保坐标不超出屏幕
        [abs_x.clamp(0, SCREEN_WIDTH), abs_y.clamp(0, SCREEN_HEIGHT)]
    }

    /// 计算容器子节点的布局位置（权重布局）
    /// - direction: 容器方向（水平/垂直）
    /// - children: 子节点列表
    /// - container_bounds: 容器边界
    pub fn calculate_child_layout(
        &self,
        direction: ContainerDirection,
        children: &[ChildLayout],
        container_bounds: [u16; 4],
    ) -> Result<Vec<[u16; 4]>> {
        let mut child_bounds = Vec::with_capacity(children.len());
        let (container_x, container_y, container_w, container_h) = (
            container_bounds[0],
            container_bounds[1],
            container_bounds[2],
            container_bounds[3],
        );

        match direction {
            // 水平布局：按权重分配宽度
            ContainerDirection::Horizontal => {
                // 计算总权重
                let total_weight: f32 = children
                    .iter()
                    .filter(|c| !c.is_absolute)
                    .map(|c| c.weight)
                    .sum();

                if total_weight <= 0.0 {
                    log::error!("容器总权重必须大于0（水平布局）");
                    return Err(AppError::RenderError);
                }

                let mut current_x = container_x;
                let child_height = container_h;

                for child in children {
                    if child.is_absolute {
                        // 绝对定位子节点：直接使用自身坐标（后续在渲染时计算）
                        child_bounds.push([0, 0, 0, 0]);
                        continue;
                    }

                    // 按权重计算子节点宽度
                    let weight = child.weight;
                    let child_width = (container_w as f32 * weight / total_weight) as u16;

                    // 确保宽度合法
                    let child_width = child_width.clamp(1, container_w);

                    child_bounds.push([current_x, container_y, child_width, child_height]);
                    current_x += child_width;

                    // 防止超出容器边界
                    if current_x > container_x + container_w {
                        break;
                    }
                }
            }

            // 垂直布局：按权重分配高度
            ContainerDirection::Vertical => {
                // 计算总权重
                let total_weight: f32 = children
                    .iter()
                    .filter(|c| !c.is_absolute)
                    .map(|c| c.weight)
                    .sum();

                if total_weight <= 0.0 {
                    log::error!("容器总权重必须大于0（垂直布局）");
                    return Err(AppError::RenderError);
                }

                let mut current_y = container_y;
                let child_width = container_w;

                for child in children {
                    if child.is_absolute {
                        // 绝对定位子节点：直接使用自身坐标
                        child_bounds.push([0, 0, 0, 0]);
                        continue;
                    }

                    // 按权重计算子节点高度
                    let weight = child.weight;
                    let child_height = (container_h as f32 * weight / total_weight) as u16;

                    // 确保高度合法
                    let child_height = child_height.clamp(1, container_h);

                    child_bounds.push([container_x, current_y, child_width, child_height]);
                    current_y += child_height;

                    // 防止超出容器边界
                    if current_y > container_y + container_h {
                        break;
                    }
                }
            }
        }

        Ok(child_bounds)
    }

    /// 创建子上下文（用于递归渲染子节点）
    pub fn create_child_context(&mut self, child_bounds: [u16; 4]) -> Result<Self> {
        if self.nest_level >= MAX_NEST_LEVEL {
            log::error!("布局嵌套层级超限（最大{}）", MAX_NEST_LEVEL);
            return Err(AppError::RenderError);
        }

        Ok(Self {
            data_source_registry: self.data_source_registry,
            cache: self.cache,
            parent_bounds: child_bounds,
            needs_redraw: self.needs_redraw,
            layout_pool: self.layout_pool,
            nest_level: self.nest_level + 1,
            evaluator: self.evaluator,
        })
    }

    /// 获取布局节点（通过NodeId）
    pub fn get_node(&self, node_id: NodeId) -> Result<&LayoutNode> {
        self.layout_pool.get_node(node_id).ok_or_else(|| {
            log::error!("无效的节点ID: {}", node_id);
            AppError::RenderError
        })
    }

    /// 标记需要重绘
    pub fn mark_redraw(&mut self) {
        self.needs_redraw = true;
    }
}
