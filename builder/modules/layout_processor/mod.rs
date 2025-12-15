//! 编译期布局构建器
//! 包含YAML解析、严格校验、池化、生成布局RS文件全流程

use anyhow::{Context, Result};
use serde_yaml;
use std::collections::HashSet;
use std::fs;

// 引入外部依赖
use crate::builder::config::BuildConfig;
use crate::builder::modules::layout_processor::layout_validate::*;
use crate::builder::modules::layout_processor::layout_yaml_parser::*;
use crate::builder::utils::progress::ProgressTracker;

mod layout_validate;
mod layout_yaml_parser;

type Id = String;
type IconId = String;
type Content = String;
type Condition = String;
type FontSize = String;
type LayoutNodeVec = Vec<LayoutNode>;
type ChildLayoutVec = Vec<ChildLayout>;
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

        progress.update_progress(2, 4, "校验布局规则");
        let validation_result = LayoutValidator::validate(&yaml_node, config)?;
        if !validation_result.warnings.is_empty() {
            println!(
                "cargo:warning=⚠️  布局文件警告: {:?}",
                validation_result.warnings
            );
        }

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

    /// 将YAML节点转换为扁平化布局池（核心池化逻辑，增强规则校验）
    fn convert_to_pool(yaml_node: &YamlLayoutNode, config: &BuildConfig) -> Result<LayoutPool> {
        let mut pool = LayoutPool::default();
        let mut id_set = HashSet::new(); // 校验ID唯一性
        let root_node_id = Self::convert_yaml_node(
            yaml_node,
            &mut pool,
            &mut id_set,
            1,     // 初始嵌套层级
            false, // 父节点是否绝对定位
            config,
        )?;

        pool.root_node_id = root_node_id;
        Ok(pool)
    }

    /// 递归转换YAML节点（编译期严格校验，适配新布局规则）
    fn convert_yaml_node(
        yaml_node: &YamlLayoutNode,
        pool: &mut LayoutPool,
        id_set: &mut HashSet<String>,
        nest_level: usize,
        _parent_absolute: bool,
        config: &BuildConfig, // 字体尺寸配置依赖
    ) -> Result<NodeId> {
        // 1. 嵌套层级校验（编译期严格限制）
        if nest_level > MAX_NEST_LEVEL {
            return Err(anyhow::anyhow!(
                "节点嵌套层级超限: {} > {}",
                nest_level,
                MAX_NEST_LEVEL
            ));
        }

        match yaml_node {
            // ========== 容器节点处理（适配新布局规则：补充全量字段） ==========
            YamlLayoutNode::Container(yaml) => {
                // 2. ID唯一性+长度校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("容器ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "容器{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                let id = yaml.id.clone();

                // 3. 子节点数量校验
                if yaml.children.len() > MAX_CHILDREN_COUNT {
                    return Err(anyhow::anyhow!(
                        "容器{}子节点数量超限: {} > {}",
                        yaml.id,
                        yaml.children.len(),
                        MAX_CHILDREN_COUNT
                    ));
                }

                // 4. 转换子节点（递归+池化）
                let mut children = Vec::new();
                for yaml_child in &yaml.children {
                    let child_node_id = Self::convert_yaml_node(
                        &yaml_child.node,
                        pool,
                        id_set,
                        nest_level + 1,
                        yaml_child.is_absolute,
                        config,
                    )?;

                    // 5. 子节点权重校验（自动修正非法值）
                    let weight = normalize_weight(yaml_child.weight);
                    validate_weight(&weight)
                        .map_err(|e| anyhow::anyhow!("容器{}子节点权重校验失败: {}", yaml.id, e))?;

                    children.push(ChildLayout {
                        node_id: child_node_id,
                        weight, // 使用修正后的权重
                        is_absolute: yaml_child.is_absolute,
                    });
                }

                // 6. 解析容器属性（适配新布局规则）
                let direction = yaml
                    .direction
                    .as_deref()
                    .map(ContainerDirection::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}方向解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();

                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(TextAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(VerticalAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}垂直对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();

                // 条件字符串校验（规则1.1）
                let condition = if let Some(cond) = &yaml.condition {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "容器{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    // 校验条件表达式语法（规则4.3）
                    validate_condition_syntax(cond)
                        .with_context(|| format!("容器{}条件表达式语法错误", yaml.id))?;
                    Some(cond.clone())
                } else {
                    None
                };

                // 解析新布局属性：position/anchor/width/height（替代原rect）
                let position = yaml.position.unwrap_or_default();
                let anchor = yaml
                    .anchor
                    .as_deref()
                    .map(Anchor::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}锚点解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();
                let width = yaml.width;
                let height = yaml.height;
                let is_absolute = yaml.is_absolute.unwrap_or(false);

                // 坐标范围校验（区分绝对/相对定位，规则3.4）
                validate_coordinate(&position, "容器", &yaml.id, is_absolute)?;
                if let Some(w) = width {
                    if w > MAX_DIMENSION {
                        return Err(anyhow::anyhow!(
                            "容器{}宽度超限: {} > {}",
                            yaml.id,
                            w,
                            MAX_DIMENSION
                        ));
                    }
                }
                if let Some(h) = height {
                    if h > MAX_DIMENSION {
                        return Err(anyhow::anyhow!(
                            "容器{}高度超限: {} > {}",
                            yaml.id,
                            h,
                            MAX_DIMENSION
                        ));
                    }
                }

                // 7. 创建容器节点并加入池（适配新结构）
                let container = LayoutNode::Container(Container {
                    id: id.clone(),
                    position,
                    anchor,
                    width,
                    height,
                    children: children.into(), // 转换为静态数组
                    condition,
                    direction,
                    alignment,
                    vertical_alignment,
                });

                let node_id = pool.nodes.len() as NodeId;
                pool.nodes.push(container);
                Ok(node_id)
            }

            // ========== 文本节点处理（适配新布局规则：补充全量字段） ==========
            YamlLayoutNode::Text(yaml) => {
                // ID唯一性+长度校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("文本ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "文本{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                let id = yaml.id.clone();

                // 内容长度校验（规则1.1）
                if yaml.content.len() > MAX_CONTENT_LENGTH {
                    return Err(anyhow::anyhow!(
                        "文本{}内容长度超限: {} > {}",
                        yaml.id,
                        yaml.content.len(),
                        MAX_CONTENT_LENGTH
                    ));
                }
                // 内容占位符校验（规则4.2）
                validate_content_placeholders(&yaml.content)
                    .with_context(|| format!("文本{}内容占位符格式错误", yaml.id))?;
                let content = yaml.content.clone();

                // 字体尺寸解析+校验（规则4.5）
                let font_size = validate_font_size(&yaml.font_size, config)
                    .with_context(|| format!("文本{}字体尺寸校验失败", yaml.id))?;

                // 对齐方式解析（使用默认值）
                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(TextAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本{}对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(VerticalAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本{}垂直对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();

                // max_lines校验
                let max_lines = if let Some(lines) = yaml.max_lines {
                    if lines < MIN_MAX_LINES {
                        Some(MIN_MAX_LINES)
                    } else if lines > MAX_MAX_LINES {
                        Some(MAX_MAX_LINES)
                    } else {
                        Some(lines)
                    }
                } else {
                    None
                };

                // max_width校验（规则2.1）
                let max_width: Option<u16> = if let Some(w) = yaml.max_width {
                    if w > SCREEN_WIDTH {
                        Some(SCREEN_WIDTH)
                    } else {
                        Some(w)
                    }
                } else {
                    None
                };

                // 条件字符串校验（规则2.3/4.3）
                let condition = if let Some(cond) = &yaml.condition {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "文本{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_syntax(cond)
                        .with_context(|| format!("文本{}条件表达式语法错误", yaml.id))?;
                    Some(cond.clone())
                } else {
                    None
                };

                // 权重校验（自动修正，规则4.6）
                let weight = normalize_weight(yaml.weight);
                validate_weight(&weight).with_context(|| format!("文本{}权重校验失败", yaml.id))?;

                // 解析新布局属性（替代rect）
                let position = yaml.position.unwrap_or_default();
                let anchor = yaml
                    .anchor
                    .as_deref()
                    .map(Anchor::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本{}锚点解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();
                let width = yaml.width;
                let height = yaml.height;
                let is_absolute = yaml.is_absolute;

                // 坐标范围校验（区分绝对/相对定位，规则3.4）
                validate_coordinate(&position, "文本", &yaml.id, is_absolute)?;
                if let Some(w) = width {
                    if w > MAX_DIMENSION {
                        return Err(anyhow::anyhow!(
                            "文本{}宽度超限: {} > {}",
                            yaml.id,
                            w,
                            MAX_DIMENSION
                        ));
                    }
                }
                if let Some(h) = height {
                    if h > MAX_DIMENSION {
                        return Err(anyhow::anyhow!(
                            "文本{}高度超限: {} > {}",
                            yaml.id,
                            h,
                            MAX_DIMENSION
                        ));
                    }
                }

                // 创建文本节点（适配新结构）
                let text = LayoutNode::Text(Text {
                    id: id.clone(),
                    position,
                    anchor,
                    width,
                    height,
                    content,
                    font_size,
                    alignment,
                    vertical_alignment,
                    max_width,
                    max_lines,
                    condition,
                    is_absolute,
                    weight,
                });

                let node_id = pool.nodes.len() as NodeId;
                pool.nodes.push(text);
                Ok(node_id)
            }

            // ========== 图标节点处理（适配新布局规则：补充全量字段） ==========
            YamlLayoutNode::Icon(yaml) => {
                // ID唯一性+长度校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("图标ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "图标{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                let id = yaml.id.clone();

                // 图标资源ID校验（规则4.4：{模块}:{键}格式）
                if !yaml.icon_id.contains(':') {
                    return Err(anyhow::anyhow!(
                        "图标{}资源ID格式错误，需符合{{模块}}:{{键}}: {}",
                        yaml.id,
                        yaml.icon_id
                    ));
                }
                if yaml.icon_id.len() > MAX_ICON_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "图标{}资源ID长度超限: {} > {}",
                        yaml.id,
                        yaml.icon_id.len(),
                        MAX_ICON_ID_LENGTH
                    ));
                }
                let icon_id = yaml.icon_id.clone();

                // 重要程度解析
                let importance = yaml
                    .importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("图标{}重要程度解析失败: {}", yaml.id, e))?;

                // 条件字符串校验（规则2.3/4.3）
                let condition = if let Some(cond) = &yaml.condition {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "图标{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_syntax(cond)
                        .with_context(|| format!("图标{}条件表达式语法错误", yaml.id))?;
                    Some(cond.clone())
                } else {
                    None
                };

                // 对齐方式解析（使用默认值）
                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(TextAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("图标{}对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(VerticalAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("图标{}垂直对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();

                // 权重校验（自动修正，规则4.6）
                let weight = normalize_weight(yaml.weight);
                validate_weight(&weight).with_context(|| format!("图标{}权重校验失败", yaml.id))?;

                // 解析新布局属性（替代rect）
                let position = yaml.position.unwrap_or_default();
                let anchor = yaml
                    .anchor
                    .as_deref()
                    .map(Anchor::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("图标{}锚点解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();
                // 图标宽高兜底（规则3.1.2）
                let width = yaml.width.or(Some(DEFAULT_ICON_WIDTH));
                let height = yaml.height.or(Some(DEFAULT_ICON_HEIGHT));
                let is_absolute = yaml.is_absolute;

                // 坐标范围校验（区分绝对/相对定位，规则3.4）
                validate_coordinate(&position, "图标", &yaml.id, is_absolute)?;
                if let Some(w) = width {
                    if w > MAX_DIMENSION {
                        return Err(anyhow::anyhow!(
                            "图标{}宽度超限: {} > {}",
                            yaml.id,
                            w,
                            MAX_DIMENSION
                        ));
                    }
                }
                if let Some(h) = height {
                    if h > MAX_DIMENSION {
                        return Err(anyhow::anyhow!(
                            "图标{}高度超限: {} > {}",
                            yaml.id,
                            h,
                            MAX_DIMENSION
                        ));
                    }
                }

                // 创建图标节点（适配新结构）
                let icon = LayoutNode::Icon(Icon {
                    id: id.clone(),
                    position,
                    anchor,
                    width,
                    height,
                    icon_id,
                    importance,
                    condition,
                    is_absolute,
                    alignment,
                    vertical_alignment,
                    weight,
                });

                let node_id = pool.nodes.len() as NodeId;
                pool.nodes.push(icon);
                Ok(node_id)
            }

            // ========== 线条节点处理（适配新布局规则：补充全量字段） ==========
            YamlLayoutNode::Line(yaml) => {
                // ID唯一性+长度校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("线条ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "线条{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                let id = yaml.id.clone();

                // 厚度校验（自动修正，规则4.7）
                let thickness = normalize_thickness(yaml.thickness);
                validate_thickness(&thickness)
                    .with_context(|| format!("线条{}厚度校验失败", yaml.id))?;

                // 重要程度解析
                let importance = yaml
                    .importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("线条{}重要程度解析失败: {}", yaml.id, e))?;

                // 条件字符串校验（规则2.3/4.3）
                let condition = if let Some(cond) = &yaml.condition {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "线条{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_syntax(cond)
                        .with_context(|| format!("线条{}条件表达式语法错误", yaml.id))?;
                    Some(cond.clone())
                } else {
                    None
                };

                let is_absolute = yaml.is_absolute;

                // 绝对坐标校验（起点+终点，规则3.4）
                validate_coordinate(&yaml.start, "线条起点", &yaml.id, is_absolute)?;
                validate_coordinate(&yaml.end, "线条终点", &yaml.id, is_absolute)?;

                // 创建线条节点
                let line = LayoutNode::Line(Line {
                    id: id.clone(),
                    start: yaml.start,
                    end: yaml.end,
                    thickness, // 使用修正后的厚度
                    importance,
                    condition,
                    is_absolute,
                });

                let node_id = pool.nodes.len() as NodeId;
                pool.nodes.push(line);
                Ok(node_id)
            }

            // ========== 矩形节点处理（适配新布局规则：补充全量字段） ==========
            YamlLayoutNode::Rectangle(yaml) => {
                // ID唯一性+长度校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("矩形ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "矩形{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                let id = yaml.id.clone();

                // 描边厚度校验（自动修正，规则4.7）
                let stroke_thickness = normalize_thickness(yaml.stroke_thickness);
                validate_thickness(&stroke_thickness)
                    .with_context(|| format!("矩形{}描边厚度校验失败", yaml.id))?;

                // 重要程度解析
                let fill_importance = yaml
                    .fill_importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("矩形{}填充重要程度解析失败: {}", yaml.id, e))?;

                let stroke_importance = yaml
                    .stroke_importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("矩形{}描边重要程度解析失败: {}", yaml.id, e))?;

                // 条件字符串校验（规则2.3/4.3）
                let condition = if let Some(cond) = &yaml.condition {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "矩形{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_syntax(cond)
                        .with_context(|| format!("矩形{}条件表达式语法错误", yaml.id))?;
                    Some(cond.clone())
                } else {
                    None
                };

                let is_absolute = yaml.is_absolute;

                // 解析新布局属性（替代rect）
                let position = yaml.position.unwrap_or_default();
                let anchor = yaml
                    .anchor
                    .as_deref()
                    .map(Anchor::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("矩形{}锚点解析失败: {}", yaml.id, e))?
                    .unwrap_or_default();
                let width = yaml.width;
                let height = yaml.height;

                // 范围校验（区分绝对/相对定位，规则3.4）
                validate_coordinate(&position, "矩形", &yaml.id, is_absolute)?;
                if width > MAX_DIMENSION || height > MAX_DIMENSION {
                    return Err(anyhow::anyhow!(
                        "矩形{}尺寸超限: 宽{} 高{} (最大{})",
                        yaml.id,
                        width,
                        height,
                        MAX_DIMENSION
                    ));
                }

                // 创建矩形节点（适配新结构）
                let rectangle = LayoutNode::Rectangle(Rectangle {
                    id: id.clone(),
                    position,
                    anchor,
                    width,
                    height,
                    fill_importance,
                    stroke_importance,
                    stroke_thickness, // 使用修正后的厚度
                    condition,
                    is_absolute,
                });

                let node_id = pool.nodes.len() as NodeId;
                pool.nodes.push(rectangle);
                Ok(node_id)
            }

            // ========== 圆形节点处理（适配新布局规则：补充全量字段） ==========
            YamlLayoutNode::Circle(yaml) => {
                // ID唯一性+长度校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("圆形ID重复: {}", yaml.id));
                }
                if yaml.id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "圆形{}ID长度超限: {} > {}",
                        yaml.id,
                        yaml.id.len(),
                        MAX_ID_LENGTH
                    ));
                }
                let id = yaml.id.clone();

                // 描边厚度校验（自动修正，规则4.7）
                let stroke_thickness = normalize_thickness(yaml.stroke_thickness);
                validate_thickness(&stroke_thickness)
                    .with_context(|| format!("圆形{}描边厚度校验失败", yaml.id))?;

                // 半径校验（自动修正，规则3.5.2）
                let radius = if yaml.radius < MIN_RADIUS {
                    MIN_RADIUS
                } else if yaml.radius > MAX_RADIUS {
                    MAX_RADIUS
                } else {
                    yaml.radius
                };

                // 重要程度解析
                let fill_importance = yaml
                    .fill_importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("圆形{}填充重要程度解析失败: {}", yaml.id, e))?;

                let stroke_importance = yaml
                    .stroke_importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("圆形{}描边重要程度解析失败: {}", yaml.id, e))?;

                // 条件字符串校验（规则2.3/4.3）
                let condition = if let Some(cond) = &yaml.condition {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "圆形{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    validate_condition_syntax(cond)
                        .with_context(|| format!("圆形{}条件表达式语法错误", yaml.id))?;
                    Some(cond.clone())
                } else {
                    None
                };

                let is_absolute = yaml.is_absolute;

                // 解析新布局属性（替代center）
                let position = yaml.position.unwrap_or_default();
                let anchor = yaml
                    .anchor
                    .as_deref()
                    .map(Anchor::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("圆形{}锚点解析失败: {}", yaml.id, e))?
                    .unwrap_or(Anchor::Center); // 圆形默认锚点为中心

                // 绝对坐标校验（规则3.4）
                validate_coordinate(&position, "圆形", &yaml.id, is_absolute)?;

                // 创建圆形节点（适配新结构）
                let circle = LayoutNode::Circle(Circle {
                    id: id.clone(),
                    position,
                    anchor,
                    radius, // 使用修正后的半径
                    fill_importance,
                    stroke_importance,
                    stroke_thickness, // 使用修正后的厚度
                    condition,
                    is_absolute,
                });

                let node_id = pool.nodes.len() as NodeId;
                pool.nodes.push(circle);
                Ok(node_id)
            }
        }
    }

    /// 生成布局RS文件（直接构造LayoutPool实例）
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

    /// 生成布局RS代码（适配新布局规则：补充全量字段）
    fn generate_layout_rs_code(pool: &LayoutPool) -> Result<String> {
        let mut code = String::new();

        // ========== 1. 头部注释 + 导入 ==========
        code.push_str("//! 自动生成的布局数据（编译期生成，运行时只读）\n");
        code.push_str("//! 不要手动修改此文件\n");
        code.push('\n');
        code.push_str("use crate::kernel::render::layout::nodes::*;\n");
        code.push('\n');
        code.push_str("// ========== 静态常量定义（运行时不可修改） ==========\n");
        code.push('\n');

        // ========== 2. 生成布局节点数组（核心） ==========
        code.push_str("/// 所有布局节点静态数组\n");
        code.push_str("#[rustfmt::skip]\n");
        code.push_str("static LAYOUT_NODES: &[LayoutNode] = &[\n");

        // 遍历所有节点生成代码
        for node in &pool.nodes {
            let node_code = Self::layout_node_to_code(node)?;
            code.push_str(&node_code);
            code.push_str(",\n");
        }

        code.push_str("];\n");
        code.push('\n');

        // ========== 4. 生成全局布局池 ==========
        code.push_str("// ========== 全局布局池（运行时只读） ==========\n");
        code.push('\n');
        code.push_str("/// 预构建的全局布局池\n");
        code.push_str("pub static MAIN_LAYOUT_POOL: LayoutPool = LayoutPool {\n");
        code.push_str(&format!("    root_node_id: {},\n", pool.root_node_id));
        code.push_str("    nodes: LAYOUT_NODES,\n");
        code.push_str("};\n");
        code.push('\n');
        code.push_str("/// 获取全局布局池（运行时直接使用）\n");
        code.push_str("pub fn get_global_layout_pool() -> &'static LayoutPool {\n");
        code.push_str("    &MAIN_LAYOUT_POOL\n");
        code.push_str("}\n");

        Ok(code)
    }

    /// 将LayoutNode转换为RS代码字面量（适配新布局规则：补充全量字段）
    fn layout_node_to_code(node: &LayoutNode) -> Result<String> {
        let mut code = String::new();

        match node {
            LayoutNode::Container(container) => {
                // ========== 容器节点（适配新属性） ==========
                code.push_str("LayoutNode::Container(Container {\n");
                code.push_str(&format!("    id: \"{}\",\n", container.id));
                code.push_str(&format!("    position: {:?},\n", container.position));
                code.push_str(&format!(
                    "    anchor: Anchor::{},\n",
                    anchor_to_code(&container.anchor)
                ));
                code.push_str(&format!("    width: {:?},\n", container.width));
                code.push_str(&format!("    height: {:?},\n", container.height));

                // 内联子节点数组
                code.push_str("    children: &[\n");
                for child in &container.children {
                    code.push_str("        ChildLayout {\n");
                    code.push_str(&format!("            node_id: {},\n", child.node_id));
                    code.push_str(&format!("            weight: {:.1},\n", child.weight));
                    code.push_str(&format!(
                        "            is_absolute: {},\n",
                        child.is_absolute
                    ));
                    code.push_str("        },\n");
                }
                code.push_str("    ],\n");

                // 容器其他属性
                code.push_str(&format!(
                    "    condition: {},\n",
                    condition_to_code(&container.condition)
                ));
                code.push_str(&format!(
                    "    direction: ContainerDirection::{},\n",
                    direction_to_code(&container.direction)
                ));
                code.push_str(&format!(
                    "    alignment: TextAlignment::{},\n",
                    alignment_to_code(&container.alignment)
                ));
                code.push_str(&format!(
                    "    vertical_alignment: VerticalAlignment::{},\n",
                    vertical_alignment_to_code(&container.vertical_alignment)
                ));
                code.push_str("})");
            }

            LayoutNode::Text(text) => {
                // ========== 文本节点（适配新属性） ==========
                code.push_str("LayoutNode::Text(Text {\n");
                code.push_str(&format!("    id: \"{}\",\n", text.id));
                code.push_str(&format!("    position: {:?},\n", text.position));
                code.push_str(&format!(
                    "    anchor: Anchor::{},\n",
                    anchor_to_code(&text.anchor)
                ));
                code.push_str(&format!("    width: {:?},\n", text.width));
                code.push_str(&format!("    height: {:?},\n", text.height));
                code.push_str(&format!(
                    "    content: \"{}\",\n",
                    text.content.replace('"', "\\\"")
                ));
                code.push_str(&format!("    font_size: FontSize::{},\n", text.font_size));
                code.push_str(&format!(
                    "    alignment: TextAlignment::{},\n",
                    alignment_to_code(&text.alignment)
                ));
                code.push_str(&format!(
                    "    vertical_alignment: VerticalAlignment::{},\n",
                    vertical_alignment_to_code(&text.vertical_alignment)
                ));
                code.push_str(&format!("    max_width: {:?},\n", text.max_width));
                code.push_str(&format!("    max_lines: {:?},\n", text.max_lines));
                code.push_str(&format!(
                    "    condition: {},\n",
                    condition_to_code(&text.condition)
                ));
                code.push_str(&format!("    is_absolute: {},\n", text.is_absolute));
                code.push_str(&format!("    weight: {:.1},\n", text.weight));
                code.push_str("})");
            }

            LayoutNode::Icon(icon) => {
                // ========== 图标节点（适配新属性） ==========
                code.push_str("LayoutNode::Icon(Icon {\n");
                code.push_str(&format!("    id: \"{}\",\n", icon.id));
                code.push_str(&format!("    position: {:?},\n", icon.position));
                code.push_str(&format!(
                    "    anchor: Anchor::{},\n",
                    anchor_to_code(&icon.anchor)
                ));
                code.push_str(&format!("    width: {:?},\n", icon.width));
                code.push_str(&format!("    height: {:?},\n", icon.height));
                code.push_str(&format!(
                    "    icon_id: \"{}\",\n",
                    icon.icon_id.replace('"', "\\\"")
                ));
                code.push_str(&format!(
                    "    importance: {},\n",
                    importance_to_code(&icon.importance)
                ));
                code.push_str(&format!(
                    "    condition: {},\n",
                    condition_to_code(&icon.condition)
                ));
                code.push_str(&format!("    is_absolute: {},\n", icon.is_absolute));
                code.push_str(&format!(
                    "    alignment: TextAlignment::{},\n",
                    alignment_to_code(&icon.alignment)
                ));
                code.push_str(&format!(
                    "    vertical_alignment: VerticalAlignment::{},\n",
                    vertical_alignment_to_code(&icon.vertical_alignment)
                ));
                code.push_str(&format!("    weight: {:.1},\n", icon.weight));
                code.push_str("})");
            }

            LayoutNode::Line(line) => {
                // ========== 线条节点（补充新字段） ==========
                code.push_str("LayoutNode::Line(Line {\n");
                code.push_str(&format!("    id: \"{}\",\n", line.id));
                code.push_str(&format!("    start: {:?},\n", line.start));
                code.push_str(&format!("    end: {:?},\n", line.end));
                code.push_str(&format!("    thickness: {},\n", line.thickness));
                code.push_str(&format!(
                    "    importance: {},\n",
                    importance_to_code(&line.importance)
                ));
                code.push_str(&format!(
                    "    condition: {},\n",
                    condition_to_code(&line.condition)
                ));
                code.push_str(&format!("    is_absolute: {},\n", line.is_absolute));
                code.push_str("})");
            }

            LayoutNode::Rectangle(rect) => {
                // ========== 矩形节点（补充新字段） ==========
                code.push_str("LayoutNode::Rectangle(Rectangle {\n");
                code.push_str(&format!("    id: \"{}\",\n", rect.id));
                code.push_str(&format!("    position: {:?},\n", rect.position));
                code.push_str(&format!(
                    "    anchor: Anchor::{},\n",
                    anchor_to_code(&rect.anchor)
                ));
                code.push_str(&format!("    width: {},\n", rect.width));
                code.push_str(&format!("    height: {},\n", rect.height));
                code.push_str(&format!(
                    "    fill_importance: {},\n",
                    importance_to_code(&rect.fill_importance)
                ));
                code.push_str(&format!(
                    "    stroke_importance: {},\n",
                    importance_to_code(&rect.stroke_importance)
                ));
                code.push_str(&format!(
                    "    stroke_thickness: {},\n",
                    rect.stroke_thickness
                ));
                code.push_str(&format!(
                    "    condition: {},\n",
                    condition_to_code(&rect.condition)
                ));
                code.push_str(&format!("    is_absolute: {},\n", rect.is_absolute));
                code.push_str("})");
            }

            LayoutNode::Circle(circle) => {
                // ========== 圆形节点（补充新字段） ==========
                code.push_str("LayoutNode::Circle(Circle {\n");
                code.push_str(&format!("    id: \"{}\",\n", circle.id));
                code.push_str(&format!("    position: {:?},\n", circle.position));
                code.push_str(&format!(
                    "    anchor: Anchor::{},\n",
                    anchor_to_code(&circle.anchor)
                ));
                code.push_str(&format!("    radius: {},\n", circle.radius));
                code.push_str(&format!(
                    "    fill_importance: {},\n",
                    importance_to_code(&circle.fill_importance)
                ));
                code.push_str(&format!(
                    "    stroke_importance: {},\n",
                    importance_to_code(&circle.stroke_importance)
                ));
                code.push_str(&format!(
                    "    stroke_thickness: {},\n",
                    circle.stroke_thickness
                ));
                code.push_str(&format!(
                    "    condition: {},\n",
                    condition_to_code(&circle.condition)
                ));
                code.push_str(&format!("    is_absolute: {},\n", circle.is_absolute));
                code.push_str("})");
            }
        }

        Ok(code)
    }
}

// ==================== 辅助函数（代码生成） ====================
fn anchor_to_code(anchor: &Anchor) -> &str {
    match anchor {
        Anchor::TopLeft => "TopLeft",
        Anchor::TopCenter => "TopCenter",
        Anchor::TopRight => "TopRight",
        Anchor::CenterLeft => "CenterLeft",
        Anchor::Center => "Center",
        Anchor::CenterRight => "CenterRight",
        Anchor::BottomLeft => "BottomLeft",
        Anchor::BottomCenter => "BottomCenter",
        Anchor::BottomRight => "BottomRight",
    }
}

fn alignment_to_code(alignment: &TextAlignment) -> &str {
    match alignment {
        TextAlignment::Left => "Left",
        TextAlignment::Center => "Center",
        TextAlignment::Right => "Right",
    }
}

fn vertical_alignment_to_code(alignment: &VerticalAlignment) -> &str {
    match alignment {
        VerticalAlignment::Top => "Top",
        VerticalAlignment::Center => "Center",
        VerticalAlignment::Bottom => "Bottom",
    }
}

fn direction_to_code(direction: &ContainerDirection) -> &str {
    match direction {
        ContainerDirection::Horizontal => "Horizontal",
        ContainerDirection::Vertical => "Vertical",
    }
}

fn importance_to_code(importance: &Option<Importance>) -> String {
    match importance {
        Some(imp) => format!(
            "Some(Importance::{})",
            match imp {
                Importance::Normal => "Normal",
                Importance::Warning => "Warning",
                Importance::Critical => "Critical",
            }
        ),
        None => "None".to_string(),
    }
}

fn condition_to_code(condition: &Option<Condition>) -> String {
    match condition {
        Some(cond) => format!("Some(\"{}\")", cond.replace('"', "\\\"")),
        None => "None".to_string(),
    }
}

/// 编译期构建布局数据（对外接口）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    LayoutBuilder::build(config, progress)
}
