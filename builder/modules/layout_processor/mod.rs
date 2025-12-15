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

mod layout_types_build_impl;
mod layout_validate;
mod layout_yaml_parser;

include!("../../../shared/layout_types.rs");
type Id = String;
type IconId = String;
type Content = String;
type Condition = String;
type FontSize = String;
type LayoutNodeVec = Vec<LayoutNode>;
type ChildLayoutVec = Vec<ChildLayout>;

pub const MAX_ID_LENGTH: usize = 64; // ID最大长度
pub const MAX_CONTENT_LENGTH: usize = 128; // 文本内容最大长度
pub const MAX_CONDITION_LENGTH: usize = 128; // 条件字符串最大长度

/// 布局构建器（仅编译期使用）
pub struct LayoutBuilder;

impl LayoutBuilder {
    /// 编译期构建布局数据（入口函数）
    pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
        progress.update_progress(0, 3, "读取布局文件");
        let yaml_content = Self::read_layout_file(config)?;

        progress.update_progress(1, 3, "解析YAML并校验");
        let yaml_node: YamlLayoutNode =
            serde_yaml::from_str(&yaml_content).with_context(|| "解析YAML布局文件失败")?;

        progress.update_progress(2, 3, "转换为扁平化布局池并生成RS文件");
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
        let mut pool = LayoutPool::new();
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

    /// 递归转换YAML节点（编译期严格校验）
    /// 递归转换YAML节点（编译期严格校验）
    fn convert_yaml_node(
        yaml_node: &YamlLayoutNode,
        pool: &mut LayoutPool,
        id_set: &mut HashSet<String>,
        nest_level: usize,
        parent_absolute: bool,
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
            // ========== 容器节点处理 ==========
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
                let mut children = Vec::new(); // 临时存储，生成代码时转为静态数组
                for yaml_child in &yaml.children {
                    let child_node_id = Self::convert_yaml_node(
                        &yaml_child.node,
                        pool,
                        id_set,
                        nest_level + 1,
                        yaml_child.is_absolute.unwrap_or(false),
                        config,
                    )?;

                    // 5. 子节点权重校验
                    if let Some(weight) = yaml_child.weight {
                        validate_weight(&weight).map_err(|e| {
                            anyhow::anyhow!("容器{}子节点权重校验失败: {}", yaml.id, e)
                        })?;
                    }

                    children.push(ChildLayout {
                        node_id: child_node_id,
                        weight: yaml_child.weight,
                        is_absolute: yaml_child.is_absolute.unwrap_or(false),
                    });
                }

                // 6. 解析容器属性
                let direction = yaml
                    .direction
                    .as_deref()
                    .map(ContainerDirection::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}方向解析失败: {}", yaml.id, e))?
                    .unwrap_or(ContainerDirection::Horizontal);

                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(TextAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(TextAlignment::Left);

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(VerticalAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器{}垂直对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(VerticalAlignment::Top);

                // 条件字符串校验
                let condition = if let Some(cond) = &yaml.condition {
                    if cond.len() > MAX_CONDITION_LENGTH {
                        return Err(anyhow::anyhow!(
                            "容器{}条件字符串长度超限: {} > {}",
                            yaml.id,
                            cond.len(),
                            MAX_CONDITION_LENGTH
                        ));
                    }
                    Some(cond.clone())
                } else {
                    None
                };

                let rect = yaml.rect.unwrap_or([0, 0, 0, 0]);

                // 7. 创建容器节点并加入池
                let container = LayoutNode::Container(Container {
                    id,
                    rect,
                    children,
                    condition,
                    direction,
                    alignment,
                    vertical_alignment,
                });

                pool.add_node(container)
                    .map_err(|e| anyhow::anyhow!("添加容器{}节点失败: {}", yaml.id, e))
            }

            // ========== 文本节点处理 ==========
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

                // 内容长度校验
                if yaml.content.len() > MAX_CONTENT_LENGTH {
                    return Err(anyhow::anyhow!(
                        "文本{}内容长度超限: {} > {}",
                        yaml.id,
                        yaml.content.len(),
                        MAX_CONTENT_LENGTH
                    ));
                }
                let content = yaml.content.clone();

                // 字体尺寸解析（从BuildConfig校验）
                let font_size_input = yaml.font_size.clone();
                let valid_font_sizes: Vec<&String> =
                    config.font_size_configs.iter().map(|fs| &fs.name).collect();
                let is_valid = config
                    .font_size_configs
                    .iter()
                    .any(|fs| fs.name.eq_ignore_ascii_case(&font_size_input));

                let font_size = if is_valid {
                    font_size_input
                } else {
                    let default_font = "Medium".to_string();
                    if !config
                        .font_size_configs
                        .iter()
                        .any(|fs| fs.name == default_font)
                    {
                        return Err(anyhow::anyhow!(
                            "文本{}默认字体尺寸{}不在配置列表中，合法值：{:?}",
                            yaml.id,
                            default_font,
                            valid_font_sizes
                        ));
                    }
                    default_font
                }
                .to_string();

                // 对齐方式解析
                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(TextAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本{}对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(TextAlignment::Left);

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(VerticalAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本{}垂直对齐解析失败: {}", yaml.id, e))?
                    .unwrap_or(VerticalAlignment::Top);

                // max_lines校验
                if let Some(max_lines) = yaml.max_lines
                    && (!(MIN_MAX_LINES..=MAX_MAX_LINES).contains(&max_lines)) {
                        return Err(anyhow::anyhow!(
                            "文本{}最大行数超限: {} (需{}~{})",
                            yaml.id,
                            max_lines,
                            MIN_MAX_LINES,
                            MAX_MAX_LINES
                        ));
                    }

                // max_width校验
                if let Some(max_width) = yaml.max_width
                    && max_width > SCREEN_WIDTH {
                        return Err(anyhow::anyhow!(
                            "文本{}最大宽度超限: {} > {}",
                            yaml.id,
                            max_width,
                            SCREEN_WIDTH
                        ));
                    }

                // 绝对坐标校验（如果父节点是绝对定位）
                if parent_absolute {
                    validate_absolute_coord(&yaml.rect, false)
                        .map_err(|e| anyhow::anyhow!("文本{}绝对坐标校验失败: {}", yaml.id, e))?;
                }

                // 创建文本节点
                let text = LayoutNode::Text(Text {
                    id,
                    rect: yaml.rect,
                    content,
                    font_size,
                    alignment,
                    vertical_alignment,
                    max_width: yaml.max_width,
                    max_lines: yaml.max_lines,
                });

                pool.add_node(text)
                    .map_err(|e| anyhow::anyhow!("添加文本{}节点失败: {}", yaml.id, e))
            }

            // ========== 图标节点处理 ==========
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

                // 图标资源ID长度校验
                if yaml.icon_id.len() > MAX_ID_LENGTH {
                    return Err(anyhow::anyhow!(
                        "图标{}资源ID长度超限: {} > {}",
                        yaml.id,
                        yaml.icon_id.len(),
                        MAX_ID_LENGTH
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

                // 绝对坐标校验
                if parent_absolute {
                    validate_absolute_coord(&yaml.rect, false)
                        .map_err(|e| anyhow::anyhow!("图标{}绝对坐标校验失败: {}", yaml.id, e))?;
                }

                // 创建图标节点
                let icon = LayoutNode::Icon(Icon {
                    id,
                    rect: yaml.rect,
                    icon_id,
                    importance,
                });

                pool.add_node(icon)
                    .map_err(|e| anyhow::anyhow!("添加图标{}节点失败: {}", yaml.id, e))
            }

            // ========== 线条节点处理 ==========
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

                // 厚度校验
                validate_thickness(&yaml.thickness)
                    .map_err(|e| anyhow::anyhow!("线条{}厚度校验失败: {}", yaml.id, e))?;

                // 重要程度解析
                let importance = yaml
                    .importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("线条{}重要程度解析失败: {}", yaml.id, e))?;

                // 绝对坐标校验（起点+终点）
                if parent_absolute {
                    validate_absolute_coord(&yaml.start, true).map_err(|e| {
                        anyhow::anyhow!("线条{}起点绝对坐标校验失败: {}", yaml.id, e)
                    })?;
                    validate_absolute_coord(&yaml.end, true).map_err(|e| {
                        anyhow::anyhow!("线条{}终点绝对坐标校验失败: {}", yaml.id, e)
                    })?;
                }

                // 创建线条节点
                let line = LayoutNode::Line(Line {
                    id,
                    start: yaml.start,
                    end: yaml.end,
                    thickness: yaml.thickness,
                    importance,
                });

                pool.add_node(line)
                    .map_err(|e| anyhow::anyhow!("添加线条{}节点失败: {}", yaml.id, e))
            }

            // ========== 矩形节点处理 ==========
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

                // 描边厚度校验
                validate_thickness(&yaml.stroke_thickness)
                    .map_err(|e| anyhow::anyhow!("矩形{}描边厚度校验失败: {}", yaml.id, e))?;

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

                // 绝对坐标校验
                if parent_absolute {
                    validate_absolute_coord(&yaml.rect, false)
                        .map_err(|e| anyhow::anyhow!("矩形{}绝对坐标校验失败: {}", yaml.id, e))?;
                }

                // 创建矩形节点
                let rectangle = LayoutNode::Rectangle(Rectangle {
                    id,
                    rect: yaml.rect,
                    fill_importance,
                    stroke_importance,
                    stroke_thickness: yaml.stroke_thickness,
                });

                pool.add_node(rectangle)
                    .map_err(|e| anyhow::anyhow!("添加矩形{}节点失败: {}", yaml.id, e))
            }

            // ========== 圆形节点处理 ==========
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

                // 描边厚度校验
                validate_thickness(&yaml.stroke_thickness)
                    .map_err(|e| anyhow::anyhow!("圆形{}描边厚度校验失败: {}", yaml.id, e))?;

                // 半径校验（避免越界）
                if yaml.radius > SCREEN_WIDTH / 2 || yaml.radius > SCREEN_HEIGHT / 2 {
                    return Err(anyhow::anyhow!(
                        "圆形{}半径超限: {} (最大{})",
                        yaml.id,
                        yaml.radius,
                        SCREEN_WIDTH.min(SCREEN_HEIGHT) / 2
                    ));
                }

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

                // 绝对坐标校验（圆心）
                if parent_absolute {
                    validate_absolute_coord(&yaml.center, true).map_err(|e| {
                        anyhow::anyhow!("圆形{}圆心绝对坐标校验失败: {}", yaml.id, e)
                    })?;
                }

                // 创建圆形节点
                let circle = LayoutNode::Circle(Circle {
                    id,
                    center: yaml.center,
                    radius: yaml.radius,
                    fill_importance,
                    stroke_importance,
                    stroke_thickness: yaml.stroke_thickness,
                });

                pool.add_node(circle)
                    .map_err(|e| anyhow::anyhow!("添加圆形{}节点失败: {}", yaml.id, e))
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
    /// 生成布局RS代码
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

        // ========== 3. 生成全局布局池（移除id_map） ==========
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

    /// 将LayoutNode转换为RS代码字面量
    fn layout_node_to_code(node: &LayoutNode) -> Result<String> {
        let mut code = String::new();

        match node {
            LayoutNode::Container(container) => {
                // ========== 容器节点：内联子节点数组，不生成单独静态常量 ==========
                code.push_str("LayoutNode::Container(Container {\n");
                code.push_str(&format!("    id: \"{}\",\n", container.id));
                code.push_str(&format!("    rect: {:?},\n", container.rect));

                // 内联子节点数组（核心修改：不再生成CONTAINER_X_CHILDREN静态变量）
                code.push_str("    children: &[\n");
                for child in &container.children {
                    code.push_str("        ChildLayout {\n");
                    code.push_str(&format!("            node_id: {},\n", child.node_id));
                    code.push_str(&format!("            weight: {:?},\n", child.weight));
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
                    match &container.condition {
                        Some(cond) => format!("Some(\"{}\")", cond.replace('"', "\\\"")),
                        None => "None".to_string(),
                    }
                ));
                code.push_str(&format!(
                    "    direction: ContainerDirection::{},\n",
                    match container.direction {
                        ContainerDirection::Horizontal => "Horizontal",
                        ContainerDirection::Vertical => "Vertical",
                    }
                ));
                code.push_str(&format!(
                    "    alignment: TextAlignment::{},\n",
                    match container.alignment {
                        TextAlignment::Left => "Left",
                        TextAlignment::Center => "Center",
                        TextAlignment::Right => "Right",
                    }
                ));
                code.push_str(&format!(
                    "    vertical_alignment: VerticalAlignment::{},\n",
                    match container.vertical_alignment {
                        VerticalAlignment::Top => "Top",
                        VerticalAlignment::Center => "Center",
                        VerticalAlignment::Bottom => "Bottom",
                    }
                ));
                code.push_str("})");
            }

            LayoutNode::Text(text) => {
                // ========== 文本节点 ==========
                code.push_str("LayoutNode::Text(Text {\n");
                code.push_str(&format!("    id: \"{}\",\n", text.id));
                code.push_str(&format!("    rect: {:?},\n", text.rect));
                code.push_str(&format!(
                    "    content: \"{}\",\n",
                    text.content.replace('"', "\\\"")
                ));
                code.push_str(&format!("    font_size: FontSize::{},\n", text.font_size));
                code.push_str(&format!(
                    "    alignment: TextAlignment::{},\n",
                    match text.alignment {
                        TextAlignment::Left => "Left",
                        TextAlignment::Center => "Center",
                        TextAlignment::Right => "Right",
                    }
                ));
                code.push_str(&format!(
                    "    vertical_alignment: VerticalAlignment::{},\n",
                    match text.vertical_alignment {
                        VerticalAlignment::Top => "Top",
                        VerticalAlignment::Center => "Center",
                        VerticalAlignment::Bottom => "Bottom",
                    }
                ));
                code.push_str(&format!("    max_width: {:?},\n", text.max_width));
                code.push_str(&format!("    max_lines: {:?},\n", text.max_lines));
                code.push_str("})");
            }

            LayoutNode::Icon(icon) => {
                // ========== 图标节点 ==========
                code.push_str("LayoutNode::Icon(Icon {\n");
                code.push_str(&format!("    id: \"{}\",\n", icon.id));
                code.push_str(&format!("    rect: {:?},\n", icon.rect));
                code.push_str(&format!(
                    "    icon_id: \"{}\",\n",
                    icon.icon_id.replace('"', "\\\"")
                ));
                code.push_str(&format!(
                    "    importance: {},\n",
                    match icon.importance {
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
                ));
                code.push_str("})");
            }

            LayoutNode::Line(line) => {
                // ========== 线条节点 ==========
                code.push_str("LayoutNode::Line(Line {\n");
                code.push_str(&format!("    id: \"{}\",\n", line.id));
                code.push_str(&format!("    start: {:?},\n", line.start));
                code.push_str(&format!("    end: {:?},\n", line.end));
                code.push_str(&format!("    thickness: {},\n", line.thickness));
                code.push_str(&format!(
                    "    importance: {},\n",
                    match line.importance {
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
                ));
                code.push_str("})");
            }

            LayoutNode::Rectangle(rect) => {
                // ========== 矩形节点 ==========
                code.push_str("LayoutNode::Rectangle(Rectangle {\n");
                code.push_str(&format!("    id: \"{}\",\n", rect.id));
                code.push_str(&format!("    rect: {:?},\n", rect.rect));
                code.push_str(&format!(
                    "    fill_importance: {},\n",
                    match rect.fill_importance {
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
                ));
                code.push_str(&format!(
                    "    stroke_importance: {},\n",
                    match rect.stroke_importance {
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
                ));
                code.push_str(&format!(
                    "    stroke_thickness: {},\n",
                    rect.stroke_thickness
                ));
                code.push_str("})");
            }

            LayoutNode::Circle(circle) => {
                // ========== 圆形节点 ==========
                code.push_str("LayoutNode::Circle(Circle {\n");
                code.push_str(&format!("    id: \"{}\",\n", circle.id));
                code.push_str(&format!("    center: {:?},\n", circle.center));
                code.push_str(&format!("    radius: {},\n", circle.radius));
                code.push_str(&format!(
                    "    fill_importance: {},\n",
                    match circle.fill_importance {
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
                ));
                code.push_str(&format!(
                    "    stroke_importance: {},\n",
                    match circle.stroke_importance {
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
                ));
                code.push_str(&format!(
                    "    stroke_thickness: {},\n",
                    circle.stroke_thickness
                ));
                code.push_str("})");
            }
        }

        Ok(code)
    }
}

/// 编译期构建布局数据（对外接口）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    LayoutBuilder::build(config, progress)
}
