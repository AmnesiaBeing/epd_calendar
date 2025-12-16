//! 布局引擎核心
//! 负责布局计算、坐标转换、权重分配、渲染调度

use heapless::Vec;

use crate::kernel::{data::DynamicValue, render::layout::nodes::*};
use crate::{
    common::error::{AppError, Result},
    kernel::data::{DataSourceRegistry, types::HeaplessString},
};

/// 布局引擎（核心）
#[derive(Debug, Clone)]
pub struct LayoutEngine {
    /// 布局节点池（扁平化存储）
    pub nodes: Vec<(NodeId, NodeId, LayoutNode), { super::MAX_NODE_COUNT as usize }>,
    /// 表达式评估器
    pub evaluator: ExpressionEvaluator,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            evaluator: ExpressionEvaluator,
        }
    }
}

impl LayoutEngine {
    /// 添加节点到池
    pub fn add_node(
        &mut self,
        node_id: NodeId,
        parent_id: NodeId,
        node: LayoutNode,
    ) -> Result<(), LayoutError> {
        self.nodes
            .push((node_id, parent_id, node))
            .map_err(|_| LayoutError::NodeNotFound(node_id))
    }

    /// 获取节点（按ID）
    pub fn get_node(&self, node_id: NodeId) -> Result<&LayoutNode, LayoutError> {
        self.nodes
            .iter()
            .find(|(id, _, _)| *id == node_id)
            .map(|(_, _, node)| node)
            .ok_or(LayoutError::NodeNotFound(node_id))
    }

    /// 获取可变节点（按ID）
    pub fn get_node_mut(&mut self, node_id: NodeId) -> Result<&mut LayoutNode, LayoutError> {
        self.nodes
            .iter_mut()
            .find(|(id, _, _)| *id == node_id)
            .map(|(_, _, node)| node)
            .ok_or(LayoutError::NodeNotFound(node_id))
    }

    /// 获取子节点（按父ID）
    pub fn get_children(&self, parent_id: NodeId) -> Vec<NodeId, 10> {
        let mut children = Vec::new();
        for (id, pid, _) in &self.nodes {
            if *pid == parent_id {
                let _ = children.push(*id);
            }
        }
        children
    }

    /// 计算所有节点的绝对坐标
    pub fn calculate_layout<'a, D>(
        &mut self,
        ctx: &mut LayoutContext<'a, D>,
    ) -> Result<(), LayoutError>
    where
        D: DataSourceRegistry,
    {
        // 从根节点开始递归计算
        self.calculate_node_layout(ctx, ctx.root_node_id, 0)?;
        Ok(())
    }

    /// 递归计算单个节点的布局
    fn calculate_node_layout<'a, D>(
        &mut self,
        ctx: &mut LayoutContext<'a, D>,
        node_id: NodeId,
        depth: u8,
    ) -> Result<(), LayoutError>
    where
        D: DataSourceRegistry,
    {
        // 检查嵌套层级
        if depth > 10 {
            return Err(LayoutError::NestedTooDeep);
        }

        // 获取节点
        let node = self.get_node_mut(node_id)?;
        let parent_id = self.get_parent_id(node_id)?;

        // 评估条件表达式，确定节点是否可见
        let condition = node.condition().as_str();
        let node_state = if condition.is_empty() {
            NodeState::Visible
        } else {
            match self.evaluator.evaluate_condition(condition, ctx) {
                Ok(true) => NodeState::Visible,
                Ok(false) => NodeState::Invisible,
                Err(e) => {
                    ctx.cache.set_node_state(node_id, NodeState::Error)?;
                    return Err(e.into());
                }
            }
        };
        ctx.cache.set_node_state(node_id, node_state)?;

        // 不可见节点跳过计算
        if node_state != NodeState::Visible {
            return Ok(());
        }

        // 计算节点尺寸
        self.calculate_node_size(node_id)?;

        // 计算节点绝对坐标
        let (parent_x, parent_y) = self.get_parent_coords(ctx, parent_id)?;
        let (node_x, node_y) = self.calculate_node_coords(ctx, node_id, parent_x, parent_y)?;
        ctx.cache.set_node_coords(node_id, (node_x, node_y))?;

        // 处理子节点
        if let LayoutNode::Container(container) = node {
            // 计算容器子节点的布局
            self.calculate_container_children(ctx, node_id, container, depth + 1)?;
        }

        Ok(())
    }

    /// 计算节点尺寸（按布局规则）
    fn calculate_node_size(&mut self, node_id: NodeId) -> Result<(), LayoutError> {
        let node = self.get_node_mut(node_id)?;

        match node {
            LayoutNode::Text(text) => {
                // Text尺寸：显式设置 > 自动计算（内容+字体）
                if text.width == 1 || text.height == 1 {
                    let font_height = match text.font_size {
                        FontSize::Small => 16,
                        FontSize::Medium => 24,
                        FontSize::Large => 40,
                    };
                    let text_width = text.content.len() as u16
                        * match text.font_size {
                            FontSize::Small => 16,
                            FontSize::Medium => 24,
                            FontSize::Large => 40,
                        };

                    text.width = text.max_width.unwrap_or(text_width).max(1);
                    text.height = text.max_height.unwrap_or(font_height).max(1);
                }
            }

            LayoutNode::Icon(icon) => {
                // Icon尺寸：显式设置 > icon_id匹配的默认尺寸
                if icon.width == 1 || icon.height == 1 {
                    let (width, height) = self.get_icon_default_size(&icon.icon_id)?;
                    icon.width = width;
                    icon.height = height;
                }
            }

            LayoutNode::Container(container) => {
                // Container尺寸：显式设置 > 子元素尺寸总和/最大值
                if container.width == 1 || container.height == 1 {
                    let children = self.get_children(node_id);
                    let mut total_width = 0;
                    let mut total_height = 0;
                    let mut max_width = 0;
                    let mut max_height = 0;

                    for child_id in children {
                        let child = self.get_node(child_id)?;
                        let (w, h) = child.size();
                        total_width += w;
                        total_height += h;
                        max_width = max_width.max(w);
                        max_height = max_height.max(h);
                    }

                    // 水平方向：宽度=子元素总和，高度=子元素最大高度
                    // 垂直方向：高度=子元素总和，宽度=子元素最大宽度
                    if container.direction == Direction::Horizontal {
                        container.width = total_width.max(1);
                        container.height = max_height.max(1);
                    } else {
                        container.width = max_width.max(1);
                        container.height = total_height.max(1);
                    }
                }
            }

            LayoutNode::Line(_) => {} // Line无尺寸计算
            LayoutNode::Rectangle(rect) => {
                // Rectangle尺寸强制≥1
                rect.width = rect.width.max(1);
                rect.height = rect.height.max(1);
            }
        }

        // 校验尺寸≥1
        let (w, h) = node.size();
        if w < 1 || h < 1 {
            return Err(LayoutError::InvalidSize);
        }

        Ok(())
    }

    /// 获取图标默认尺寸（匹配BuildConfig规则）
    fn get_icon_default_size(
        &self,
        icon_id: &HeaplessString<super::ICON_ID_LENGTH>,
    ) -> Result<(u16, u16), LayoutError> {
        let icon_id_str = icon_id.as_str();
        let parts: Vec<&str> = icon_id_str.split(':').collect();
        if parts.len() != 2 {
            return Err(LayoutError::InvalidNodeType);
        }

        let module = parts[0];
        let size = match module {
            "time_digit" => (48, 64),
            "battery" | "network" => (32, 32),
            "weather" => (64, 64),
            _ => return Err(LayoutError::InvalidNodeType),
        };

        Ok(size)
    }

    /// 计算节点绝对坐标（锚点转换+父容器偏移）
    fn calculate_node_coords<'a, D>(
        &self,
        ctx: &mut LayoutContext<'a, D>,
        node_id: NodeId,
        parent_x: u16,
        parent_y: u16,
    ) -> Result<(u16, u16), LayoutError>
    where
        D: DataSourceRegistry,
    {
        let node = self.get_node(node_id)?;
        let (w, h) = node.size();

        // 获取节点相对父容器的坐标（i16运算坐标）
        let (rel_x, rel_y) = match node {
            LayoutNode::Container(n) => n.position,
            LayoutNode::Text(n) => n.position,
            LayoutNode::Icon(n) => n.position,
            LayoutNode::Line(n) => n.start, // Line用start作为基准
            LayoutNode::Rectangle(n) => n.position,
        };

        // 锚点转换（编译期规则）
        let (anchor_x, anchor_y) = match node {
            LayoutNode::Container(n) => self.convert_anchor(n.anchor, rel_x, rel_y, w, h),
            LayoutNode::Icon(n) => self.convert_anchor(n.anchor, rel_x, rel_y, w, h),
            LayoutNode::Rectangle(n) => self.convert_anchor(n.anchor, rel_x, rel_y, w, h),
            LayoutNode::Text(_) => (rel_x, rel_y), // Text无anchor
            LayoutNode::Line(_) => (rel_x, rel_y), // Line无anchor
        };

        // 转换为绝对坐标（父容器偏移 + 锚点坐标）
        let abs_x = (parent_x as i16 + anchor_x).clamp(0, 800) as u16;
        let abs_y = (parent_y as i16 + anchor_y).clamp(0, 480) as u16;

        Ok((abs_x, abs_y))
    }

    /// 锚点坐标转换（匹配布局规则）
    fn convert_anchor(&self, anchor: Anchor, x: i16, y: i16, w: u16, h: u16) -> (i16, i16) {
        match anchor {
            Anchor::TopLeft => (x, y),
            Anchor::TopCenter => (x - (w / 2) as i16, y),
            Anchor::TopRight => (x - w as i16, y),
            Anchor::CenterLeft => (x, y - (h / 2) as i16),
            Anchor::Center => (x - (w / 2) as i16, y - (h / 2) as i16),
            Anchor::CenterRight => (x - w as i16, y - (h / 2) as i16),
            Anchor::BottomLeft => (x, y - h as i16),
            Anchor::BottomCenter => (x - (w / 2) as i16, y - h as i16),
            Anchor::BottomRight => (x - w as i16, y - h as i16),
        }
    }

    /// 计算容器子节点的布局（权重分配）
    fn calculate_container_children<'a, D>(
        &mut self,
        ctx: &mut LayoutContext<'a, D>,
        container_id: NodeId,
        container: &ContainerNode,
        depth: u8,
    ) -> Result<(), LayoutError>
    where
        D: DataSourceRegistry,
    {
        let children = self.get_children(container_id);
        let container_size = if container.direction == Direction::Horizontal {
            container.width
        } else {
            container.height
        };

        // 筛选参与权重分配的子节点（weight>0，layout=Flow）
        let mut weight_children = Vec::new();
        let mut total_fixed_size = 0;
        let mut total_weight = 0.0;

        for child_id in &children {
            let child = self.get_node(*child_id)?;
            let child_weight = match child {
                LayoutNode::Container(n) => n.weight,
                LayoutNode::Text(n) => n.weight,
                LayoutNode::Icon(n) => n.weight,
                _ => 0.0, // Line/Rectangle不参与权重分配
            };

            if child.layout() == Layout::Flow && child_weight > 0.0 {
                let _ = weight_children.push((*child_id, child_weight));
                total_weight += child_weight;
            } else {
                // 非权重子节点：累加固定尺寸
                let child_size = if container.direction == Direction::Horizontal {
                    child.size().0
                } else {
                    child.size().1
                };
                total_fixed_size += child_size;
            }
        }

        // 计算剩余空间
        let remaining_space = if container_size > total_fixed_size {
            container_size - total_fixed_size
        } else {
            0
        };

        // 权重分配（嵌入式适配：f32→u16，四舍五入）
        let mut current_offset = 0;
        for (child_id, weight) in weight_children {
            let child = self.get_node_mut(child_id)?;
            let allocated_size = if total_weight > 0.0 {
                ((remaining_space as f32) * (weight / total_weight)).round() as u16
            } else {
                0
            };

            // 设置子节点尺寸
            let (w, h) = child.size();
            if container.direction == Direction::Horizontal {
                child.set_size(allocated_size.max(1), h);
            } else {
                child.set_size(w, allocated_size.max(1));
            }

            // 设置子节点相对坐标
            let (rel_x, rel_y) = match child {
                LayoutNode::Container(n) => &mut n.position,
                LayoutNode::Text(n) => &mut n.position,
                LayoutNode::Icon(n) => &mut n.position,
                _ => continue,
            };

            if container.direction == Direction::Horizontal {
                *rel_x = current_offset as i16;
                *rel_y = match container.vertical_alignment {
                    Alignment::Start => 0,
                    Alignment::Center => ((container.height - h) / 2) as i16,
                    Alignment::End => (container.height - h) as i16,
                };
                current_offset += allocated_size;
            } else {
                *rel_x = match container.alignment {
                    Alignment::Start => 0,
                    Alignment::Center => ((container.width - w) / 2) as i16,
                    Alignment::End => (container.width - w) as i16,
                };
                *rel_y = current_offset as i16;
                current_offset += allocated_size;
            }

            // 递归计算子节点布局
            self.calculate_node_layout(ctx, child_id, depth)?;
        }

        // 处理非权重子节点
        for child_id in children {
            let child = self.get_node(child_id)?;
            if child.layout() == Layout::Absolute
                || !weight_children.iter().any(|(id, _)| *id == child_id)
            {
                self.calculate_node_layout(ctx, child_id, depth)?;
            }
        }

        Ok(())
    }

    /// 获取父节点ID
    fn get_parent_id(&self, node_id: NodeId) -> Result<NodeId, LayoutError> {
        self.nodes
            .iter()
            .find(|(id, _, _)| *id == node_id)
            .map(|(_, pid, _)| *pid)
            .ok_or(LayoutError::NodeNotFound(node_id))
    }

    /// 获取父节点坐标
    fn get_parent_coords<'a, D>(
        &self,
        ctx: &mut LayoutContext<'a, D>,
        parent_id: NodeId,
    ) -> Result<(u16, u16), LayoutError>
    where
        D: DataSourceRegistry,
    {
        if parent_id == ROOT_PARENT_ID {
            // 根节点父坐标为(0,0)
            Ok((0, 0))
        } else {
            ctx.cache
                .get_node_coords(parent_id)
                .ok_or(LayoutError::NodeNotFound(parent_id))
        }
    }
}
