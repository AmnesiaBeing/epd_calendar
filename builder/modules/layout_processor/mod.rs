//! 编译期布局构建器
//! 包含YAML解析、严格校验、池化、生成布局RS文件全流程

use anyhow::{Context, Result};
use serde_yaml;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;

// 引入外部依赖
use crate::builder::config::BuildConfig;
use crate::builder::modules::layout_processor::layout_validate::*;
use crate::builder::modules::layout_processor::layout_yaml_parser::*;
use crate::builder::utils::progress::ProgressTracker;

mod layout_validate;
mod layout_yaml_parser;

type NodeIdStr = String;
type IconId = String;
type Content = String;
type Condition = String;
type FontSize = String;
type LayoutNodeEntryVec = Vec<LayoutPoolEntry>;
type ChildLayoutVec = Vec<NodeId>;
include!("../../../shared/layout_types.rs");

/// 布局构建器（仅编译期使用）
pub struct LayoutBuilder;

impl LayoutBuilder {
    /// 编译期构建布局数据（入口函数）
    pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
        progress.update_progress(0, 4, "读取布局文件"); // 调整进度步长
        let yaml_content = Self::read_layout_file(config)?;

        progress.update_progress(1, 4, "解析YAML");
        let yaml_node: YamlLayoutNode =
            serde_yaml::from_str(&yaml_content).with_context(|| "解析YAML布局文件失败")?;

        progress.update_progress(3, 4, "转换为扁平化布局池并生成RS文件");
        let layout_pool = Self::convert_to_pool(&yaml_node, config)?;
        Self::write_layout_rs_file(config, &layout_pool)?;

        Ok(())
    }

    /// 读取YAML布局文件
    fn read_layout_file(config: &BuildConfig) -> Result<String> {
        let layout_path = &config.main_layout_path;
        if !layout_path.exists() {
            return Err(anyhow::anyhow!("布局文件不存在: {}", layout_path.display()));
        }

        fs::read_to_string(layout_path)
            .with_context(|| format!("读取布局文件失败: {}", layout_path.display()))
    }

    /// 将YAML节点转换为扁平化布局池（核心池化逻辑）
    fn convert_to_pool(yaml_node: &YamlLayoutNode, config: &BuildConfig) -> Result<LayoutPool> {
        let mut pool = LayoutPool::default();
        let mut id_set = HashSet::new(); // 校验ID唯一性
        let mut node_id_counter = 0u16;
        let mut node_map = HashMap::new(); // 存储id到NodeId的映射
        let mut parent_map = HashMap::new(); // 存储节点到父节点的映射

        // 先分配所有节点的ID
        Self::assign_node_ids(
            yaml_node,
            &mut node_id_counter,
            &mut node_map,
            &mut parent_map,
            ROOT_PARENT_ID,
            &mut id_set,
        )?;

        // 转换所有节点（递归处理）
        Self::convert_all_nodes(yaml_node, &mut pool, &node_map, &parent_map, config)?;

        // 设置根节点ID（应该是第一个分配的节点）
        pool.root_node_id = 0;

        Ok(pool)
    }

    /// 分配节点ID（先序遍历）
    fn assign_node_ids(
        yaml_node: &YamlLayoutNode,
        counter: &mut NodeId,
        node_map: &mut HashMap<String, NodeId>,
        parent_map: &mut HashMap<NodeId, NodeId>,
        parent_id: NodeId,
        id_set: &mut HashSet<String>,
    ) -> Result<()> {
        let current_id = *counter;

        // 获取节点ID字符串并进行校验
        let node_id_str = match yaml_node {
            YamlLayoutNode::Container(yaml) => {
                // ID唯一性校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("节点ID重复: {}", yaml.id));
                }
                // 长度校验
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "容器{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                &yaml.id
            }
            YamlLayoutNode::Text(yaml) => {
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("节点ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "文本{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                &yaml.id
            }
            YamlLayoutNode::Icon(yaml) => {
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("节点ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "图标{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                &yaml.id
            }
            YamlLayoutNode::Line(yaml) => {
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("节点ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "线条{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                &yaml.id
            }
            YamlLayoutNode::Rectangle(yaml) => {
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("节点ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "矩形{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                &yaml.id
            }
        };

        // 记录映射关系
        node_map.insert(node_id_str.clone(), current_id);
        parent_map.insert(current_id, parent_id);
        *counter += 1;

        // 递归处理子节点
        if let YamlLayoutNode::Container(yaml) = yaml_node {
            // 子节点数量校验
            if yaml.children.len() > MAX_CHILDREN_COUNT {
                return Err(anyhow::anyhow!(
                    "容器{}子节点数量超限: {} > {}",
                    yaml.id,
                    yaml.children.len(),
                    MAX_CHILDREN_COUNT
                ));
            }

            for child in &yaml.children {
                Self::assign_node_ids(
                    &child.node,
                    counter,
                    node_map,
                    parent_map,
                    current_id,
                    id_set,
                )?;
            }
        }

        Ok(())
    }

    /// 转换所有节点（递归处理）
    fn convert_all_nodes(
        yaml_node: &YamlLayoutNode,
        pool: &mut LayoutPool,
        node_map: &HashMap<String, NodeId>,
        parent_map: &HashMap<NodeId, NodeId>,
        config: &BuildConfig,
    ) -> Result<()> {
        // 先转换当前节点
        Self::convert_single_node(yaml_node, pool, node_map, parent_map, config)?;

        // 递归处理子节点
        if let YamlLayoutNode::Container(yaml) = yaml_node {
            for child in &yaml.children {
                Self::convert_all_nodes(&child.node, pool, node_map, parent_map, config)?;
            }
        }

        Ok(())
    }

    /// 转换单个节点
    fn convert_single_node(
        yaml_node: &YamlLayoutNode,
        pool: &mut LayoutPool,
        node_map: &HashMap<String, NodeId>,
        parent_map: &HashMap<NodeId, NodeId>,
        config: &BuildConfig,
    ) -> Result<()> {
        match yaml_node {
            YamlLayoutNode::Container(yaml) => {
                let node_id = *node_map
                    .get(&yaml.id)
                    .ok_or_else(|| anyhow::anyhow!("容器节点ID未找到: {}", yaml.id))?;
                let parent_id = *parent_map.get(&node_id).unwrap_or(&ROOT_PARENT_ID);

                // 转换子节点ID列表
                let mut children_ids = Vec::new();
                for child in &yaml.children {
                    let child_node_id = node_map
                        .get(match &child.node {
                            YamlLayoutNode::Container(c) => &c.id,
                            YamlLayoutNode::Text(t) => &t.id,
                            YamlLayoutNode::Icon(i) => &i.id,
                            YamlLayoutNode::Line(l) => &l.id,
                            YamlLayoutNode::Rectangle(r) => &r.id,
                        })
                        .ok_or_else(|| anyhow::anyhow!("子节点ID未找到"))?;
                    children_ids.push(*child_node_id);
                }

                // 解析布局类型
                let layout = match yaml.layout.as_str() {
                    "absolute" => Layout::Absolute,
                    _ => Layout::Flow, // 默认flow
                };

                // 坐标转换（i16 -> u16）
                let position = yaml.position.map(|pos| Self::convert_coordinate(pos));

                // 锚点解析
                let anchor = if layout == Layout::Absolute {
                    yaml.anchor
                        .as_deref()
                        .map(Anchor::try_from)
                        .transpose()
                        .map_err(|e| anyhow::anyhow!("容器{}锚点解析失败: {}", yaml.id, e))?
                        .or(Some(Anchor::TopLeft))
                } else {
                    None // flow布局不需要锚点
                };

                // 方向解析
                let direction = match yaml.direction.as_str() {
                    "vertical" => Direction::Vertical,
                    _ => Direction::Horizontal, // 默认horizontal
                };

                // 对齐解析
                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(Alignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(Alignment::Start);

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(Alignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}垂直对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(Alignment::Start);

                // 条件表达式校验
                let condition = if let Some(cond) = yaml.condition.clone() {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "容器{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_expression(&cond)
                        .with_context(|| format!("容器{}条件表达式语法错误", yaml.id))?;
                    Some(cond)
                } else {
                    None
                };

                // 宽高校验
                if let Some(w) = yaml.width {
                    if w == 0 {
                        println!("cargo:warning=⚠️  容器{}宽度为0，修正为1", yaml.id);
                    } else if w > SCREEN_WIDTH {
                        return Err(anyhow::anyhow!(
                            "容器{}宽度超限: {} > {}",
                            yaml.id,
                            w,
                            SCREEN_WIDTH
                        ));
                    }
                }

                if let Some(h) = yaml.height {
                    if h == 0 {
                        println!("cargo:warning=⚠️  容器{}高度为0，修正为1", yaml.id);
                    } else if h > SCREEN_HEIGHT {
                        return Err(anyhow::anyhow!(
                            "容器{}高度超限: {} > {}",
                            yaml.id,
                            h,
                            SCREEN_HEIGHT
                        ));
                    }
                }

                let container = LayoutNode::Container(Container {
                    id: node_id,
                    #[cfg(debug_assertions)]
                    id_str: yaml.id.clone(),
                    layout,
                    position,
                    anchor,
                    direction,
                    alignment,
                    vertical_alignment,
                    children: children_ids,
                    weight: yaml.weight,
                    width: yaml.width,
                    height: yaml.height,
                    condition,
                });

                pool.nodes.push((node_id, parent_id, container));
                Ok(())
            }

            YamlLayoutNode::Text(yaml) => {
                let node_id = *node_map.get(&yaml.id).unwrap();
                let parent_id = *parent_map.get(&node_id).unwrap();

                // 布局类型
                let layout = match yaml.layout.as_str() {
                    "absolute" => Layout::Absolute,
                    _ => Layout::Flow, // 默认flow
                };

                // 坐标转换
                let position = yaml.position.map(|pos| Self::convert_coordinate(pos));

                // 字体尺寸解析
                let font_size = Self::parse_font_size(&yaml.font_size, config)
                    .with_context(|| format!("文本{}字体尺寸解析失败", yaml.id))?;

                // 对齐解析
                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(Alignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本{}对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(Alignment::Start);

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(Alignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本{}垂直对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(Alignment::Start);

                // 条件表达式校验
                let condition = if let Some(cond) = yaml.condition.clone() {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "文本{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_expression(&cond)
                        .with_context(|| format!("文本{}条件表达式语法错误", yaml.id))?;
                    Some(cond)
                } else {
                    None
                };

                // 内容截断
                let content = if yaml.content.len() > MAX_CONTENT_LENGTH {
                    // 按UTF-8完整字符截断
                    let mut truncated = String::new();
                    for ch in yaml.content.chars() {
                        if truncated.len() + ch.len_utf8() > MAX_CONTENT_LENGTH {
                            break;
                        }
                        truncated.push(ch);
                    }
                    println!(
                        "cargo:warning=⚠️  文本{}内容过长，已截断: {} -> {}字符",
                        yaml.id,
                        yaml.content.len(),
                        truncated.len()
                    );
                    truncated
                } else {
                    yaml.content.clone()
                };

                // 内容占位符校验
                validate_content_placeholders(&content)
                    .with_context(|| format!("文本{}内容占位符格式错误", yaml.id))?;

                // 宽高校验
                if let Some(w) = yaml.width {
                    if w == 0 {
                        println!("cargo:warning=⚠️  文本{}宽度为0，修正为1", yaml.id);
                    } else if w > SCREEN_WIDTH {
                        return Err(anyhow::anyhow!(
                            "文本{}宽度超限: {} > {}",
                            yaml.id,
                            w,
                            SCREEN_WIDTH
                        ));
                    }
                }

                if let Some(h) = yaml.height {
                    if h == 0 {
                        println!("cargo:warning=⚠️  文本{}高度为0，修正为1", yaml.id);
                    } else if h > SCREEN_HEIGHT {
                        return Err(anyhow::anyhow!(
                            "文本{}高度超限: {} > {}",
                            yaml.id,
                            h,
                            SCREEN_HEIGHT
                        ));
                    }
                }

                let text = LayoutNode::Text(Text {
                    id: node_id,
                    #[cfg(debug_assertions)]
                    id_str: yaml.id.clone(),
                    layout,
                    position,
                    content,
                    font_size,
                    alignment,
                    vertical_alignment,
                    max_width: yaml.max_width,
                    max_height: yaml.max_height,
                    weight: yaml.weight,
                    width: yaml.width,
                    height: yaml.height,
                    condition,
                });

                pool.nodes.push((node_id, parent_id, text));
                Ok(())
            }

            YamlLayoutNode::Icon(yaml) => {
                let node_id = *node_map
                    .get(&yaml.id)
                    .ok_or_else(|| anyhow::anyhow!("图标节点ID未找到: {}", yaml.id))?;
                let parent_id = *parent_map.get(&node_id).unwrap_or(&ROOT_PARENT_ID);

                // 布局类型
                let layout = match yaml.layout.as_str() {
                    "absolute" => Layout::Absolute,
                    _ => Layout::Flow, // 默认flow
                };

                // 坐标转换
                let position = yaml.position.map(|pos| Self::convert_coordinate(pos));

                // 锚点解析
                let anchor = if layout == Layout::Absolute {
                    yaml.anchor
                        .as_deref()
                        .map(Anchor::try_from)
                        .transpose()
                        .map_err(|e| anyhow::anyhow!("图标{}锚点解析失败: {}", yaml.id, e))?
                        .or(Some(Anchor::TopLeft))
                } else {
                    None // flow布局不需要锚点
                };

                // 对齐解析
                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(Alignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("图标{}对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(Alignment::Start);

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(Alignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("图标{}垂直对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(Alignment::Start);

                // 条件表达式校验
                let condition = if let Some(cond) = yaml.condition.clone() {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "图标{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_expression(&cond)
                        .with_context(|| format!("图标{}条件表达式语法错误", yaml.id))?;
                    Some(cond)
                } else {
                    None
                };

                // icon_id校验
                validate_icon_id(&yaml.icon_id, config)
                    .with_context(|| format!("图标{} icon_id校验失败", yaml.id))?;

                // 图标尺寸补充
                let (width, height) =
                    Self::parse_icon_size(&yaml.icon_id, yaml.width, yaml.height, config)
                        .with_context(|| format!("图标{} 尺寸解析失败", yaml.id))?;

                // 权重校验
                if let Some(weight) = yaml.weight {
                    if weight < 0.0 {
                        println!("cargo:warning=⚠️  图标{}权重为负数，修正为0.0", yaml.id);
                    } else if weight > 10.0 {
                        println!("cargo:warning=⚠️  图标{}权重大于10.0，修正为10.0", yaml.id);
                    }
                }

                let icon = LayoutNode::Icon(Icon {
                    id: node_id,
                    #[cfg(debug_assertions)]
                    id_str: yaml.id.clone(),
                    layout,
                    position,
                    anchor,
                    icon_id: yaml.icon_id.clone(),
                    alignment,
                    vertical_alignment,
                    weight: yaml.weight,
                    width: Some(width),
                    height: Some(height),
                    condition,
                });

                pool.nodes.push((node_id, parent_id, icon));
                Ok(())
            }

            YamlLayoutNode::Line(yaml) => {
                let node_id = *node_map
                    .get(&yaml.id)
                    .ok_or_else(|| anyhow::anyhow!("线条节点ID未找到: {}", yaml.id))?;
                let parent_id = *parent_map.get(&node_id).unwrap_or(&ROOT_PARENT_ID);

                // 坐标转换
                let start = Self::convert_coordinate(yaml.start);
                let end = Self::convert_coordinate(yaml.end);

                // 条件表达式校验
                let condition = if let Some(cond) = yaml.condition.clone() {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "线条{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_expression(&cond)
                        .with_context(|| format!("线条{}条件表达式语法错误", yaml.id))?;
                    Some(cond)
                } else {
                    None
                };

                // 厚度校验
                let thickness = if yaml.thickness == 0 {
                    println!("cargo:warning=⚠️  线条{}厚度为0，修正为1", yaml.id);
                    1
                } else if yaml.thickness > 10 {
                    println!(
                        "cargo:warning=⚠️  线条{}厚度过大{}，修正为10",
                        yaml.id, yaml.thickness
                    );
                    10
                } else {
                    yaml.thickness
                };

                let line = LayoutNode::Line(Line {
                    id: node_id,
                    #[cfg(debug_assertions)]
                    id_str: yaml.id.clone(),
                    thickness,
                    start,
                    end,
                    condition,
                });

                pool.nodes.push((node_id, parent_id, line));
                Ok(())
            }

            YamlLayoutNode::Rectangle(yaml) => {
                let node_id = *node_map
                    .get(&yaml.id)
                    .ok_or_else(|| anyhow::anyhow!("矩形节点ID未找到: {}", yaml.id))?;
                let parent_id = *parent_map.get(&node_id).unwrap_or(&ROOT_PARENT_ID);

                // 布局类型
                let layout = match yaml.layout.as_str() {
                    "absolute" => Layout::Absolute,
                    _ => Layout::Flow, // 默认flow
                };

                // 坐标转换
                let position = yaml.position.map(|pos| Self::convert_coordinate(pos));

                // 锚点解析
                let anchor = if layout == Layout::Absolute {
                    yaml.anchor
                        .as_deref()
                        .map(Anchor::try_from)
                        .transpose()
                        .map_err(|e| anyhow::anyhow!("矩形{}锚点解析失败: {}", yaml.id, e))?
                        .or(Some(Anchor::TopLeft))
                } else {
                    None // flow布局不需要锚点
                };

                // 条件表达式校验
                let condition = if let Some(cond) = yaml.condition.clone() {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "矩形{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_expression(&cond)
                        .with_context(|| format!("矩形{}条件表达式语法错误", yaml.id))?;
                    Some(cond)
                } else {
                    None
                };

                // 宽高校验
                let width = if let Some(w) = yaml.width {
                    if w == 0 {
                        println!("cargo:warning=⚠️  矩形{}宽度为0，修正为1", yaml.id);
                        1
                    } else if w > SCREEN_WIDTH {
                        return Err(anyhow::anyhow!(
                            "矩形{}宽度超限: {} > {}",
                            yaml.id,
                            w,
                            SCREEN_WIDTH
                        ));
                    } else {
                        w
                    }
                } else {
                    return Err(anyhow::anyhow!("矩形{}必须指定宽度", yaml.id));
                };

                let height = if let Some(h) = yaml.height {
                    if h == 0 {
                        println!("cargo:warning=⚠️  矩形{}高度为0，修正为1", yaml.id);
                        1
                    } else if h > SCREEN_HEIGHT {
                        return Err(anyhow::anyhow!(
                            "矩形{}高度超限: {} > {}",
                            yaml.id,
                            h,
                            SCREEN_HEIGHT
                        ));
                    } else {
                        h
                    }
                } else {
                    return Err(anyhow::anyhow!("矩形{}必须指定高度", yaml.id));
                };

                // 描边厚度校验
                let thickness = if yaml.thickness == 0 {
                    println!("cargo:warning=⚠️  矩形{}描边厚度为0，修正为1", yaml.id);
                    1
                } else if yaml.thickness > 10 {
                    println!(
                        "cargo:warning=⚠️  矩形{}描边厚度过大{}，修正为10",
                        yaml.id, yaml.thickness
                    );
                    10
                } else {
                    yaml.thickness
                };

                let rectangle = LayoutNode::Rectangle(Rectangle {
                    id: node_id,
                    #[cfg(debug_assertions)]
                    id_str: yaml.id.clone(),
                    layout,
                    position,
                    anchor,
                    width: Some(width),
                    height: Some(height),
                    thickness,
                    condition,
                });

                pool.nodes.push((node_id, parent_id, rectangle));
                Ok(())
            }
        }
    }

    /// 坐标转换：i16运算坐标 → u16绝对坐标
    fn convert_coordinate(coord: [i16; 2]) -> [u16; 2] {
        [
            coord[0].max(0).min(SCREEN_WIDTH as i16) as u16,
            coord[1].max(0).min(SCREEN_HEIGHT as i16) as u16,
        ]
    }

    /// 解析字体尺寸
    fn parse_font_size(font_size: &str, config: &BuildConfig) -> Result<FontSize> {
        // 查找匹配的字体尺寸配置
        for fs_config in &config.font_size_configs {
            if fs_config.name.to_lowercase() == font_size.to_lowercase() {
                return Ok(fs_config.name.clone());
            }
        }
        // 未找到，使用第一个配置
        if let Some(first) = config.font_size_configs.first() {
            println!(
                "cargo:warning=⚠️  字体尺寸'{}'未找到，使用默认'{}'",
                font_size, first.name
            );
            return Ok(first.name.clone());
        }
        Err(anyhow::anyhow!("没有可用的字体尺寸配置"))
    }

    /// 解析图标尺寸
    fn parse_icon_size(
        icon_id: &str,
        width: Option<u16>,
        height: Option<u16>,
        config: &BuildConfig,
    ) -> Result<(u16, u16)> {
        // 如果已设置宽高，直接使用
        if let (Some(w), Some(h)) = (width, height) {
            if w == 0 || h == 0 {
                return Err(anyhow::anyhow!("图标宽高不能为0: {}x{}", w, h));
            }
            return Ok((w, h));
        }

        // 解析icon_id格式：{模块}:{键}
        let parts: Vec<&str> = icon_id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "icon_id格式错误，应为{{模块}}:{{键}}: {}",
                icon_id
            ));
        }

        let module = parts[0];

        // 查找匹配的图标分类配置
        for category in &config.icon_categories {
            if category.category == module {
                if category.width == 0 || category.height == 0 {
                    return Err(anyhow::anyhow!(
                        "图标分类'{}'的宽高配置错误: {}x{}",
                        category.category,
                        category.width,
                        category.height
                    ));
                }
                return Ok((category.width, category.height));
            }
        }

        // 检查是否为天气图标
        if module == "weather" {
            if config.weather_icon_config.width == 0 || config.weather_icon_config.height == 0 {
                return Err(anyhow::anyhow!(
                    "天气图标宽高配置错误: {}x{}",
                    config.weather_icon_config.width,
                    config.weather_icon_config.height
                ));
            }
            return Ok((
                config.weather_icon_config.width,
                config.weather_icon_config.height,
            ));
        }

        Err(anyhow::anyhow!("未找到图标模块'{}'的尺寸配置", module))
    }

    /// 生成布局RS文件
    fn write_layout_rs_file(config: &BuildConfig, pool: &LayoutPool) -> Result<()> {
        let output_dir = &config.output_dir;
        fs::create_dir_all(output_dir)
            .with_context(|| format!("创建输出目录失败: {}", output_dir.display()))?;

        let rs_path = output_dir.join("generated_layouts.rs");
        let rs_code = Self::generate_layout_rs_code(pool)?;

        fs::write(&rs_path, rs_code)
            .with_context(|| format!("写入布局RS文件失败: {}", rs_path.display()))?;

        Ok(())
    }

    /// 生成布局RS代码
    fn generate_layout_rs_code(pool: &LayoutPool) -> Result<String> {
        let mut code = String::new();

        // ========== 1. 头部注释 + 导入 ==========
        code.push_str("//! 自动生成的布局数据（编译期生成，运行时只读）\n");
        code.push_str("//! 不要手动修改此文件\n\n");

        code.push_str("#![allow(dead_code)]\n");
        code.push_str("#![allow(unused_imports)]\n\n");

        code.push_str("use crate::kernel::render::layout::nodes::*;\n");

        // ========== 3. 生成布局节点数组 ==========
        code.push_str("// ========== 布局节点静态数组 ==========\n");
        code.push_str("#[rustfmt::skip]\n");
        code.push_str("static LAYOUT_NODES: &[(NodeId, NodeId, LayoutNode)] = &[\n");

        // 按node_id排序，确保顺序一致
        let mut sorted_nodes = pool.nodes.clone();
        sorted_nodes.sort_by_key(|(node_id, _, _)| *node_id);

        // 遍历所有节点生成代码
        for (node_id, parent_id, node) in &sorted_nodes {
            let node_code = Self::layout_node_to_code(node_id, parent_id, node)?;
            code.push_str(&node_code);
            code.push_str(",\n");
        }

        code.push_str("];\n\n");

        // ========== 4. 生成根节点ID常量 ==========
        code.push_str("// ========== 根节点标识 ==========\n");
        code.push_str(&format!(
            "pub const ROOT_NODE_ID: NodeId = {};\n\n",
            pool.root_node_id
        ));

        // ========== 5. 生成全局布局池 ==========
        code.push_str("// ========== 全局布局池（运行时只读） ==========\n");
        code.push_str("pub static MAIN_LAYOUT_POOL: LayoutPool = LayoutPool {\n");
        code.push_str(&format!("    root_node_id: {},\n", pool.root_node_id));
        code.push_str("    nodes: LAYOUT_NODES,\n");
        code.push_str("};\n\n");

        code.push_str("/// 获取全局布局池（运行时直接使用）\n");
        code.push_str("pub fn get_global_layout_pool() -> &'static LayoutPool {\n");
        code.push_str("    &MAIN_LAYOUT_POOL\n");
        code.push_str("}\n");

        Ok(code)
    }

    /// 将LayoutNode转换为RS代码字面量
    fn layout_node_to_code(
        node_id: &NodeId,
        parent_id: &NodeId,
        node: &LayoutNode,
    ) -> Result<String> {
        let mut code = String::new();

        // 开始元组
        code.push_str(&format!("({}, {}, ", node_id, parent_id));

        match node {
            LayoutNode::Container(container) => {
                code.push_str("LayoutNode::Container(Container {\n");
                code.push_str(&format!("    id: {},\n", container.id));
                #[cfg(debug_assertions)]
                {
                    code.push_str(&format!("    id_str: \"{}\",\n", container.id_str));
                }
                code.push_str(&format!(
                    "    layout: Layout::{},\n",
                    match container.layout {
                        Layout::Flow => "Flow",
                        Layout::Absolute => "Absolute",
                    }
                ));
                code.push_str(&format!("    position: {:?},\n", container.position));
                code.push_str(&format!("    anchor: {:?},\n", container.anchor));
                code.push_str(&format!(
                    "    direction: Direction::{},\n",
                    match container.direction {
                        Direction::Horizontal => "Horizontal",
                        Direction::Vertical => "Vertical",
                    }
                ));
                code.push_str(&format!(
                    "    alignment: Alignment::{},\n",
                    match container.alignment {
                        Alignment::Start => "Start",
                        Alignment::Center => "Center",
                        Alignment::End => "End",
                    }
                ));
                code.push_str(&format!(
                    "    vertical_alignment: Alignment::{},\n",
                    match container.vertical_alignment {
                        Alignment::Start => "Start",
                        Alignment::Center => "Center",
                        Alignment::End => "End",
                    }
                ));
                code.push_str("    children: &[\n");
                for child_id in container.children.iter() {
                    code.push_str(&format!("        {},\n", child_id));
                }
                code.push_str("    ],\n");
                code.push_str(&format!("    weight: {:?},\n", container.weight));
                code.push_str(&format!("    width: {:?},\n", container.width));
                code.push_str(&format!("    height: {:?},\n", container.height));
                code.push_str(&format!("    condition: {:?},\n", container.condition));
                code.push_str("})");
            }

            LayoutNode::Text(text) => {
                code.push_str("LayoutNode::Text(Text {\n");
                code.push_str(&format!("    id: {},\n", text.id));
                #[cfg(debug_assertions)]
                {
                    code.push_str(&format!("    id_str: \"{}\",\n", text.id_str));
                }
                code.push_str(&format!(
                    "    layout: Layout::{},\n",
                    match text.layout {
                        Layout::Flow => "Flow",
                        Layout::Absolute => "Absolute",
                    }
                ));
                code.push_str(&format!("    position: {:?},\n", text.position));
                code.push_str(&format!(
                    "    content: \"{}\",\n",
                    text.content.replace('"', "\\\"")
                ));
                code.push_str(&format!("    font_size: FontSize::{},\n", text.font_size));
                code.push_str(&format!(
                    "    alignment: Alignment::{},\n",
                    match text.alignment {
                        Alignment::Start => "Start",
                        Alignment::Center => "Center",
                        Alignment::End => "End",
                    }
                ));
                code.push_str(&format!(
                    "    vertical_alignment: Alignment::{},\n",
                    match text.vertical_alignment {
                        Alignment::Start => "Start",
                        Alignment::Center => "Center",
                        Alignment::End => "End",
                    }
                ));
                code.push_str(&format!("    max_width: {:?},\n", text.max_width));
                code.push_str(&format!("    max_height: {:?},\n", text.max_height));
                code.push_str(&format!("    weight: {:?},\n", text.weight));
                code.push_str(&format!("    width: {:?},\n", text.width));
                code.push_str(&format!("    height: {:?},\n", text.height));
                code.push_str(&format!("    condition: {:?},\n", text.condition));
                code.push_str("})");
            }

            LayoutNode::Icon(icon) => {
                code.push_str("LayoutNode::Icon(Icon {\n");
                code.push_str(&format!("    id: {},\n", icon.id));
                #[cfg(debug_assertions)]
                {
                    code.push_str(&format!("    id_str: \"{}\",\n", icon.id_str));
                }
                code.push_str(&format!(
                    "    layout: Layout::{},\n",
                    match icon.layout {
                        Layout::Flow => "Flow",
                        Layout::Absolute => "Absolute",
                    }
                ));
                code.push_str(&format!("    position: {:?},\n", icon.position));
                code.push_str(&format!("    anchor: {:?},\n", icon.anchor));
                code.push_str(&format!(
                    "    icon_id: \"{}\",\n",
                    icon.icon_id.replace('"', "\\\"")
                ));
                code.push_str(&format!(
                    "    alignment: Alignment::{},\n",
                    match icon.alignment {
                        Alignment::Start => "Start",
                        Alignment::Center => "Center",
                        Alignment::End => "End",
                    }
                ));
                code.push_str(&format!(
                    "    vertical_alignment: Alignment::{},\n",
                    match icon.vertical_alignment {
                        Alignment::Start => "Start",
                        Alignment::Center => "Center",
                        Alignment::End => "End",
                    }
                ));
                code.push_str(&format!("    weight: {:?},\n", icon.weight));
                code.push_str(&format!("    width: {:?},\n", icon.width));
                code.push_str(&format!("    height: {:?},\n", icon.height));
                code.push_str(&format!("    condition: {:?},\n", icon.condition));
                code.push_str("})");
            }

            LayoutNode::Line(line) => {
                code.push_str("LayoutNode::Line(Line {\n");
                code.push_str(&format!("    id: {},\n", line.id));
                #[cfg(debug_assertions)]
                {
                    code.push_str(&format!("    id_str: \"{}\",\n", line.id_str));
                }
                code.push_str(&format!("    thickness: {},\n", line.thickness));
                code.push_str(&format!("    start: {:?},\n", line.start));
                code.push_str(&format!("    end: {:?},\n", line.end));
                code.push_str(&format!("    condition: {:?},\n", line.condition));
                code.push_str("})");
            }

            LayoutNode::Rectangle(rect) => {
                code.push_str("LayoutNode::Rectangle(Rectangle {\n");
                code.push_str(&format!("    id: {},\n", rect.id));
                #[cfg(debug_assertions)]
                {
                    code.push_str(&format!("    id_str: \"{}\",\n", rect.id_str));
                }
                code.push_str(&format!(
                    "    layout: Layout::{},\n",
                    match rect.layout {
                        Layout::Flow => "Flow",
                        Layout::Absolute => "Absolute",
                    }
                ));
                code.push_str(&format!("    position: {:?},\n", rect.position));
                code.push_str(&format!("    anchor: {:?},\n", rect.anchor));
                code.push_str(&format!("    width: {:?},\n", rect.width));
                code.push_str(&format!("    height: {:?},\n", rect.height));
                code.push_str(&format!("    thickness: {},\n", rect.thickness));
                code.push_str(&format!("    condition: {:?},\n", rect.condition));
                code.push_str("})");
            }
        }

        // 结束元组
        code.push_str(")");

        Ok(code)
    }
}

/// 编译期构建布局数据（对外接口）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    LayoutBuilder::build(config, progress)
}
