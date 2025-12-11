//! 编译期布局构建器
//! 包含YAML解析、严格校验、池化、序列化全流程

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashSet;
use std::fs;

// 引入外部依赖（根据项目实际路径调整）
use crate::builder::config::BuildConfig;
use crate::builder::modules::layout_processor::layout_validate::*;
use crate::builder::modules::layout_processor::layout_yaml_parser::*;
use crate::builder::utils::progress::ProgressTracker;

mod layout_types_build_impl;
mod layout_validate;
mod layout_yaml_parser;

include!("../../../shared/layout_types.rs");
include!("../../../shared/generated_font_size.rs");
include!("../../../shared/generated_font_size_impl.rs");

/// 布局构建器（仅编译期使用）
pub struct LayoutBuilder;

impl LayoutBuilder {
    /// 编译期构建布局数据（入口函数）
    pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
        progress.update_progress(0, 4, "读取布局文件");
        let yaml_content = Self::read_layout_file(config)?;

        progress.update_progress(1, 4, "解析YAML并校验");
        let yaml_node: YamlLayoutNode =
            serde_yaml::from_str(&yaml_content).with_context(|| "解析YAML布局文件失败")?;

        progress.update_progress(2, 4, "转换为扁平化布局池");
        let layout_pool = Self::convert_to_pool(&yaml_node)?;

        progress.update_progress(3, 4, "序列化并生成文件");
        let bin_data = Self::serialize_pool(&layout_pool)?;
        Self::write_output_files(config, &bin_data)?;

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
    fn convert_to_pool(yaml_node: &YamlLayoutNode) -> Result<LayoutPool> {
        let mut pool = LayoutPool::new();
        let mut id_set = HashSet::new(); // 校验ID唯一性
        let root_node_id = Self::convert_yaml_node(
            yaml_node,
            &mut pool,
            &mut id_set,
            1,     // 初始嵌套层级
            false, // 父节点是否绝对定位
        )?;

        pool.root_node_id = root_node_id;
        Ok(pool)
    }

    /// 递归转换YAML节点（编译期严格校验）
    fn convert_yaml_node(
        yaml_node: &YamlLayoutNode,
        pool: &mut LayoutPool,
        id_set: &mut HashSet<String>,
        nest_level: usize,
        parent_absolute: bool,
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
            YamlLayoutNode::Container(yaml) => {
                // 2. ID唯一性+长度校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("容器ID重复: {}", yaml.id));
                }
                let id = IdString::new(&yaml.id)
                    .map_err(|e| anyhow::anyhow!("容器ID校验失败: {}", e))?;

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
                        yaml_child.is_absolute.unwrap_or(false),
                    )?;

                    // 5. 子节点权重校验
                    if let Some(weight) = yaml_child.weight {
                        validate_weight(&weight)
                            .map_err(|e| anyhow::anyhow!("子节点权重校验失败: {}", e))?;
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
                    .map_err(|e| anyhow::anyhow!("容器方向解析失败: {}", e))?
                    .unwrap_or(ContainerDirection::Horizontal);

                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(TextAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器对齐解析失败: {}", e))?
                    .unwrap_or(TextAlignment::Left);

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(VerticalAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("容器垂直对齐解析失败: {}", e))?
                    .unwrap_or(VerticalAlignment::Top);

                let condition = yaml
                    .condition
                    .as_ref()
                    .map(|c| ConditionString::new(c))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("条件字符串校验失败: {}", e))?;

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
                    .map_err(|e| anyhow::anyhow!("添加容器节点失败: {}", e))
            }

            YamlLayoutNode::Text(yaml) => {
                // ID校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("文本ID重复: {}", yaml.id));
                }
                let id = IdString::new(&yaml.id)
                    .map_err(|e| anyhow::anyhow!("文本ID校验失败: {}", e))?;

                // 内容校验
                let content = ContentString::new(&yaml.content)
                    .map_err(|e| anyhow::anyhow!("文本内容校验失败: {}", e))?;

                // 字体尺寸解析
                let font_size = FontSize::try_from(yaml.font_size.as_str())
                    .map_err(|e| anyhow::anyhow!("字体尺寸解析失败: {}", e))
                    .unwrap_or(FontSize::Medium);

                // 对齐方式解析
                let alignment = yaml
                    .alignment
                    .as_deref()
                    .map(TextAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本对齐解析失败: {}", e))?
                    .unwrap_or(TextAlignment::Left);

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(VerticalAlignment::try_from)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本垂直对齐解析失败: {}", e))?
                    .unwrap_or(VerticalAlignment::Top);

                // max_lines校验
                if let Some(max_lines) = yaml.max_lines {
                    if max_lines < MIN_MAX_LINES || max_lines > MAX_MAX_LINES {
                        return Err(anyhow::anyhow!(
                            "文本{}最大行数超限: {} (需{}~{})",
                            yaml.id,
                            max_lines,
                            MIN_MAX_LINES,
                            MAX_MAX_LINES
                        ));
                    }
                }

                // max_width校验
                if let Some(max_width) = yaml.max_width {
                    if max_width > SCREEN_WIDTH {
                        return Err(anyhow::anyhow!(
                            "文本{}最大宽度超限: {} > {}",
                            yaml.id,
                            max_width,
                            SCREEN_WIDTH
                        ));
                    }
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
                    .map_err(|e| anyhow::anyhow!("添加文本节点失败: {}", e))
            }

            YamlLayoutNode::Icon(yaml) => {
                // ID校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("图标ID重复: {}", yaml.id));
                }
                let id = IdString::new(&yaml.id)
                    .map_err(|e| anyhow::anyhow!("图标ID校验失败: {}", e))?;

                // 图标ID校验
                let icon_id = IdString::new(&yaml.icon_id)
                    .map_err(|e| anyhow::anyhow!("图标资源ID校验失败: {}", e))?;

                // 重要程度解析
                let importance = yaml
                    .importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("图标重要程度解析失败: {}", e))?;

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
                    .map_err(|e| anyhow::anyhow!("添加图标节点失败: {}", e))
            }

            YamlLayoutNode::Line(yaml) => {
                // ID校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("线条ID重复: {}", yaml.id));
                }
                let id = IdString::new(&yaml.id)
                    .map_err(|e| anyhow::anyhow!("线条ID校验失败: {}", e))?;

                // 厚度校验
                validate_thickness(&yaml.thickness)
                    .map_err(|e| anyhow::anyhow!("线条{}厚度校验失败: {}", yaml.id, e))?;

                // 重要程度解析
                let importance = yaml
                    .importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("线条重要程度解析失败: {}", e))?;

                // 绝对坐标校验
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
                    .map_err(|e| anyhow::anyhow!("添加线条节点失败: {}", e))
            }

            YamlLayoutNode::Rectangle(yaml) => {
                // ID校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("矩形ID重复: {}", yaml.id));
                }
                let id = IdString::new(&yaml.id)
                    .map_err(|e| anyhow::anyhow!("矩形ID校验失败: {}", e))?;

                // 描边厚度校验
                validate_thickness(&yaml.stroke_thickness)
                    .map_err(|e| anyhow::anyhow!("矩形{}描边厚度校验失败: {}", yaml.id, e))?;

                // 重要程度解析
                let fill_importance = yaml
                    .fill_importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("矩形填充重要程度解析失败: {}", e))?;

                let stroke_importance = yaml
                    .stroke_importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("矩形描边重要程度解析失败: {}", e))?;

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
                    .map_err(|e| anyhow::anyhow!("添加矩形节点失败: {}", e))
            }

            YamlLayoutNode::Circle(yaml) => {
                // ID校验
                if !id_set.insert(yaml.id.clone()) {
                    return Err(anyhow::anyhow!("圆形ID重复: {}", yaml.id));
                }
                let id = IdString::new(&yaml.id)
                    .map_err(|e| anyhow::anyhow!("圆形ID校验失败: {}", e))?;

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
                    .map_err(|e| anyhow::anyhow!("圆形填充重要程度解析失败: {}", e))?;

                let stroke_importance = yaml
                    .stroke_importance
                    .as_ref()
                    .map(|i| Importance::try_from(i.as_str()))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("圆形描边重要程度解析失败: {}", e))?;

                // 绝对坐标校验
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
                    .map_err(|e| anyhow::anyhow!("添加圆形节点失败: {}", e))
            }
        }
    }

    /// 序列化布局池为二进制
    fn serialize_pool(pool: &LayoutPool) -> Result<Vec<u8>> {
        postcard::to_stdvec(pool).map_err(|e| anyhow::anyhow!("布局池序列化失败: {}", e))
    }

    /// 写入输出文件（二进制+Rust代码）
    fn write_output_files(config: &BuildConfig, bin_data: &[u8]) -> Result<()> {
        let output_dir = &config.output_dir;
        fs::create_dir_all(output_dir)
            .with_context(|| format!("创建输出目录失败: {}", output_dir.display()))?;

        // 1. 写入二进制文件
        let bin_path = output_dir.join("generated_layouts.bin");
        fs::write(&bin_path, bin_data)
            .with_context(|| format!("写入布局二进制文件失败: {}", bin_path.display()))?;

        // 2. 生成Rust代码（运行时访问用）
        let rust_path = output_dir.join("generated_layouts.rs");
        let rust_code = Self::generate_runtime_code(bin_data.len());
        fs::write(&rust_path, rust_code)
            .with_context(|| format!("写入布局Rust代码失败: {}", rust_path.display()))?;

        Ok(())
    }

    /// 生成运行时访问代码
    fn generate_runtime_code(bin_size: usize) -> String {
        format!(
            r#"//! 自动生成的布局数据（编译期生成，运行时只读）
//! 不要手动修改此文件

use crate::kernel::render::layout::nodes::LayoutPool;
use postcard::from_bytes;

/// 主布局二进制数据
pub const MAIN_LAYOUT_BIN: &[u8] = include_bytes!("generated_layouts.bin");

/// 布局数据大小（字节）
pub const LAYOUT_DATA_SIZE: usize = {};

/// 运行时加载布局池（极简校验）
pub fn load_layout_pool() -> Result<LayoutPool, &'static str> {{
    let pool: LayoutPool = from_bytes(MAIN_LAYOUT_BIN)
        .map_err(|_| "布局数据反序列化失败")?;
    
    // 运行时仅校验根节点存在
    if pool.root_node_id as usize >= pool.nodes.len() {{
        return Err("根节点ID无效");
    }}
    
    Ok(pool)
}}
"#,
            bin_size
        )
    }
}

/// 编译期构建布局数据（对外接口）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    LayoutBuilder::build(config, progress)
}
