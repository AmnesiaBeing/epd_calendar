//! 布局处理模块 - 解析YAML布局文件，验证资源合法性，生成二进制布局数据

use crate::builder::config::BuildConfig;
use crate::builder::utils::progress::ProgressTracker;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

// ==================== 常量定义 ====================

const SCREEN_WIDTH: u16 = 800;
const SCREEN_HEIGHT: u16 = 480;

// ID和字符串长度限制
const MAX_ID_LENGTH: usize = 32;
const MAX_CONTENT_LENGTH: usize = 128;
const MAX_CONDITION_LENGTH: usize = 64;
const MAX_IMPORTANCE_LENGTH: usize = 16;
const MAX_CHILDREN_COUNT: usize = 64;

// 有效的SystemState变量
const VALID_SYSTEM_STATE_FIELDS: &[&str] = &[
    "time",
    "date",
    "lunar",
    "weather",
    "quote",
    "charging_status",
    "battery_level",
    "network_status",
];

// 有效的字体尺寸
const VALID_FONT_SIZES: &[&str] = &["Small", "Medium", "Large"];

// ==================== 类型别名 ====================

type String32 = String;
type String128 = String;
type String64 = String;
type String16 = String;

// ==================== 枚举和结构体定义 ====================

/// 字体尺寸枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FontSize {
    Small,
    Medium,
    Large,
}

impl FromStr for FontSize {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Small" => Ok(FontSize::Small),
            "Medium" => Ok(FontSize::Medium),
            "Large" => Ok(FontSize::Large),
            _ => Err(format!(
                "无效字体尺寸: {}. 有效值: {}",
                s,
                VALID_FONT_SIZES.join("/")
            )),
        }
    }
}

/// 重要程度枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Importance {
    Normal,   // Black
    Warning,  // Yellow
    Critical, // Red
}

impl FromStr for Importance {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "" | "normal" | "black" => Ok(Importance::Normal),
            "warning" | "yellow" => Ok(Importance::Warning),
            "critical" | "red" => Ok(Importance::Critical),
            _ => Err(format!("无效的重要程度: {}", s)),
        }
    }
}

/// 坐标验证结果
#[derive(Debug, Clone)]
struct CoordinateBounds {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    right: u16,
    bottom: u16,
}

impl CoordinateBounds {
    fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            right: x.saturating_add(width),
            bottom: y.saturating_add(height),
        }
    }

    fn is_within_screen(&self) -> bool {
        self.x <= SCREEN_WIDTH
            && self.y <= SCREEN_HEIGHT
            && self.right <= SCREEN_WIDTH
            && self.bottom <= SCREEN_HEIGHT
    }
}

// ==================== 布局结构体定义 ====================

/// 布局节点枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    Container(Container),
    Text(Text),
    Icon(Icon),
    Line(Line),
    Rectangle(Rectangle),
    Circle(Circle),
}

impl LayoutNode {
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

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        match self {
            LayoutNode::Container(c) => c.validate(id_set),
            LayoutNode::Text(t) => t.validate(id_set),
            LayoutNode::Icon(i) => i.validate(id_set),
            LayoutNode::Line(l) => l.validate(id_set),
            LayoutNode::Rectangle(r) => r.validate(id_set),
            LayoutNode::Circle(c) => c.validate(id_set),
        }
    }
}

/// 容器元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String32,
    pub rect: [u16; 4],
    pub children: Vec<LayoutNode>,
    pub condition: Option<String64>,
}

impl Container {
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        validate_id(&self.id, id_set)?;

        let bounds = CoordinateBounds::new(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        validate_bounds(&self.id, &bounds)?;

        if let Some(condition) = &self.condition {
            validate_condition(condition)?;
        }

        if self.children.len() > MAX_CHILDREN_COUNT {
            return Err(anyhow::anyhow!(
                "容器 '{}' 的子元素数量超过限制: {} > {}",
                self.id,
                self.children.len(),
                MAX_CHILDREN_COUNT
            ));
        }

        for child in &self.children {
            child.validate(id_set)?;
        }

        Ok(())
    }
}

/// 文本元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub id: String32,
    pub position: [u16; 2],
    pub size: [u16; 2],
    pub content: String128,
    pub font_size: FontSize,
}

impl Text {
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        validate_id(&self.id, id_set)?;

        let bounds = CoordinateBounds::new(
            self.position[0],
            self.position[1],
            self.size[0],
            self.size[1],
        );
        validate_bounds(&self.id, &bounds)?;

        if self.content.len() > MAX_CONTENT_LENGTH {
            return Err(anyhow::anyhow!(
                "文本 '{}' 内容长度超过限制: {} > {}",
                self.id,
                self.content.len(),
                MAX_CONTENT_LENGTH
            ));
        }

        Ok(())
    }
}

/// 图标元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub id: String32,
    pub position: [u16; 2],
    pub icon_id: String32,
    pub importance: Option<Importance>,
}

impl Icon {
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        validate_id(&self.id, id_set)?;

        let bounds = CoordinateBounds::new(self.position[0], self.position[1], 0, 0);
        validate_bounds(&self.id, &bounds)?;

        validate_icon_id(&self.icon_id)?;

        Ok(())
    }
}

/// 线条元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub id: String32,
    pub start: [u16; 2],
    pub end: [u16; 2],
    pub thickness: u16,
    pub importance: Option<Importance>,
}

impl Line {
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        validate_id(&self.id, id_set)?;

        let start_bounds = CoordinateBounds::new(self.start[0], self.start[1], 0, 0);
        let end_bounds = CoordinateBounds::new(self.end[0], self.end[1], 0, 0);

        validate_bounds(&self.id, &start_bounds)?;
        validate_bounds(&self.id, &end_bounds)?;

        Ok(())
    }
}

/// 矩形元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub id: String32,
    pub rect: [u16; 4],
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

impl Rectangle {
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        validate_id(&self.id, id_set)?;

        let bounds = CoordinateBounds::new(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        validate_bounds(&self.id, &bounds)?;

        Ok(())
    }
}

/// 圆形元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circle {
    pub id: String32,
    pub center: [u16; 2],
    pub radius: u16,
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

impl Circle {
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        validate_id(&self.id, id_set)?;

        // 检查圆心
        let center_bounds = CoordinateBounds::new(self.center[0], self.center[1], 0, 0);
        validate_bounds(&self.id, &center_bounds)?;

        // 检查外接矩形
        let diameter = self.radius * 2;
        let bounds = CoordinateBounds::new(
            self.center[0].saturating_sub(self.radius),
            self.center[1].saturating_sub(self.radius),
            diameter,
            diameter,
        );

        if !bounds.is_within_screen() {
            return Err(anyhow::anyhow!(
                "圆形 '{}' 超出屏幕范围: 圆心({}, {}), 半径{}",
                self.id,
                self.center[0],
                self.center[1],
                self.radius
            ));
        }

        Ok(())
    }
}

// ==================== YAML解析结构体 ====================

/// YAML布局节点
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum YamlLayoutNode {
    #[serde(rename = "container")]
    Container(YamlContainer),
    #[serde(rename = "text")]
    Text(YamlText),
    #[serde(rename = "icon")]
    Icon(YamlIcon),
    #[serde(rename = "line")]
    Line(YamlLine),
    #[serde(rename = "rectangle")]
    Rectangle(YamlRectangle),
    #[serde(rename = "circle")]
    Circle(YamlCircle),
}

#[derive(Debug, Deserialize)]
struct YamlContainer {
    id: String,
    rect: Option<[u16; 4]>,
    children: Vec<YamlLayoutNode>,
    condition: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YamlText {
    id: String,
    position: [u16; 2],
    size: [u16; 2],
    content: String,
    font_size: String,
}

#[derive(Debug, Deserialize)]
struct YamlIcon {
    id: String,
    position: [u16; 2],
    icon_id: String,
    importance: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YamlLine {
    id: String,
    start: [u16; 2],
    end: [u16; 2],
    thickness: u16,
    importance: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YamlRectangle {
    id: String,
    rect: [u16; 4],
    fill_importance: Option<String>,
    stroke_importance: Option<String>,
    stroke_thickness: u16,
}

#[derive(Debug, Deserialize)]
struct YamlCircle {
    id: String,
    center: [u16; 2],
    radius: u16,
    fill_importance: Option<String>,
    stroke_importance: Option<String>,
    stroke_thickness: u16,
}

// ==================== 布局构建器 ====================

pub struct LayoutBuilder;

impl LayoutBuilder {
    /// 构建布局数据
    pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
        progress.update_progress(0, 4, "读取布局文件");
        let root_layout = Self::read_and_parse_layout(config)?;

        progress.update_progress(1, 4, "验证布局资源合法性");
        Self::validate_layout(&root_layout)?;

        progress.update_progress(2, 4, "序列化布局数据");
        let layout_bin = Self::serialize_layout(&root_layout)?;

        progress.update_progress(3, 4, "生成布局文件");
        Self::generate_layout_files(config, &layout_bin)?;

        println!("cargo:warning=  布局处理完成，成功生成布局数据");
        Ok(())
    }

    /// 读取并解析布局文件
    fn read_and_parse_layout(config: &BuildConfig) -> Result<LayoutNode> {
        let layout_path = Self::get_layout_path(config)?;
        let content = fs::read_to_string(&layout_path)
            .with_context(|| format!("读取布局文件失败: {}", layout_path.display()))?;

        let yaml_node: YamlLayoutNode = serde_yaml::from_str(&content)
            .with_context(|| format!("解析布局文件失败: {}", layout_path.display()))?;

        Self::convert_yaml_to_layout(&yaml_node)
    }

    /// 获取布局文件路径
    fn get_layout_path(config: &BuildConfig) -> Result<PathBuf> {
        // 这里可以根据配置选择不同的布局文件
        let path = Path::new("assets/layout/main.yaml");
        if !path.exists() {
            return Err(anyhow::anyhow!("布局文件不存在: {}", path.display()));
        }
        Ok(path.to_path_buf())
    }

    /// 转换YAML布局节点
    fn convert_yaml_to_layout(yaml_node: &YamlLayoutNode) -> Result<LayoutNode> {
        match yaml_node {
            YamlLayoutNode::Container(yaml) => {
                let mut children = Vec::new();
                for child in &yaml.children {
                    children.push(Self::convert_yaml_to_layout(child)?);
                }

                Ok(LayoutNode::Container(Container {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "容器ID")?,
                    rect: yaml.rect.unwrap_or([0, 0, 0, 0]),
                    children,
                    condition: yaml
                        .condition
                        .as_ref()
                        .map(|c| {
                            Self::validate_and_truncate_string(
                                c,
                                MAX_CONDITION_LENGTH,
                                "条件字符串",
                            )
                        })
                        .transpose()?,
                }))
            }

            YamlLayoutNode::Text(yaml) => {
                let font_size =
                    FontSize::from_str(&yaml.font_size).map_err(|e| anyhow::anyhow!("{}", e))?;

                Ok(LayoutNode::Text(Text {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "文本ID")?,
                    position: yaml.position,
                    size: yaml.size,
                    content: Self::validate_and_truncate_string(
                        &yaml.content,
                        MAX_CONTENT_LENGTH,
                        "文本内容",
                    )?,
                    font_size,
                }))
            }

            YamlLayoutNode::Icon(yaml) => {
                let importance = yaml
                    .importance
                    .as_ref()
                    .map(|i| Importance::from_str(i))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                Ok(LayoutNode::Icon(Icon {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "图标ID")?,
                    position: yaml.position,
                    icon_id: Self::validate_and_truncate_string(
                        &yaml.icon_id,
                        MAX_ID_LENGTH,
                        "图标资源ID",
                    )?,
                    importance,
                }))
            }

            YamlLayoutNode::Line(yaml) => {
                let importance = yaml
                    .importance
                    .as_ref()
                    .map(|i| Importance::from_str(i))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                Ok(LayoutNode::Line(Line {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "线条ID")?,
                    start: yaml.start,
                    end: yaml.end,
                    thickness: yaml.thickness,
                    importance,
                }))
            }

            YamlLayoutNode::Rectangle(yaml) => {
                let fill_importance = yaml
                    .fill_importance
                    .as_ref()
                    .map(|i| Importance::from_str(i))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                let stroke_importance = yaml
                    .stroke_importance
                    .as_ref()
                    .map(|i| Importance::from_str(i))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                Ok(LayoutNode::Rectangle(Rectangle {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "矩形ID")?,
                    rect: yaml.rect,
                    fill_importance,
                    stroke_importance,
                    stroke_thickness: yaml.stroke_thickness,
                }))
            }

            YamlLayoutNode::Circle(yaml) => {
                let fill_importance = yaml
                    .fill_importance
                    .as_ref()
                    .map(|i| Importance::from_str(i))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                let stroke_importance = yaml
                    .stroke_importance
                    .as_ref()
                    .map(|i| Importance::from_str(i))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                Ok(LayoutNode::Circle(Circle {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "圆形ID")?,
                    center: yaml.center,
                    radius: yaml.radius,
                    fill_importance,
                    stroke_importance,
                    stroke_thickness: yaml.stroke_thickness,
                }))
            }
        }
    }

    /// 验证并截断字符串
    fn validate_and_truncate_string(s: &str, max_len: usize, field_name: &str) -> Result<String> {
        if s.len() > max_len {
            // 发出警告但不失败，自动截断
            println!(
                "cargo:warning=警告: {} '{}' 长度超过限制 ({} > {})，已自动截断",
                field_name,
                s,
                s.len(),
                max_len
            );
            Ok(s.chars().take(max_len).collect())
        } else {
            Ok(s.to_string())
        }
    }

    /// 验证布局
    fn validate_layout(layout: &LayoutNode) -> Result<()> {
        let mut id_set = HashSet::new();
        layout.validate(&mut id_set)
    }

    /// 序列化布局
    fn serialize_layout(layout: &LayoutNode) -> Result<Vec<u8>> {
        postcard::to_stdvec(layout).map_err(|e| anyhow::anyhow!("序列化失败: {}", e))
    }

    /// 生成布局文件
    fn generate_layout_files(config: &BuildConfig, layout_bin: &[u8]) -> Result<()> {
        let output_dir = Path::new("src/assets");
        fs::create_dir_all(output_dir)
            .with_context(|| format!("创建输出目录失败: {}", output_dir.display()))?;

        // 写入二进制文件
        let bin_path = output_dir.join("generated_layouts.bin");
        fs::write(&bin_path, layout_bin)
            .with_context(|| format!("写入二进制文件失败: {}", bin_path.display()))?;

        // 生成Rust代码文件
        let rust_path = output_dir.join("generated_layouts.rs");
        let rust_content = Self::generate_rust_code(layout_bin.len());
        fs::write(&rust_path, rust_content)
            .with_context(|| format!("写入Rust代码失败: {}", rust_path.display()))?;

        Ok(())
    }

    /// 生成Rust代码
    fn generate_rust_code(bin_size: usize) -> String {
        format!(
            r#"//! 自动生成的布局数据文件
//! 文件大小: {} 字节
//! 生成时间: {}
//! 不要手动修改此文件

use crate::builder::modules::layout_processor::*;

/// 主布局二进制数据
pub const MAIN_LAYOUT_BIN: &[u8] = include_bytes!("generated_layouts.bin");

/// 布局数据大小（字节）
pub const LAYOUT_DATA_SIZE: usize = {};

/// 获取布局数据
#[inline]
pub fn get_layout_data() -> &'static [u8] {{
    MAIN_LAYOUT_BIN
}}
"#,
            bin_size,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            bin_size
        )
    }
}

// ==================== 验证函数 ====================

/// 验证ID
fn validate_id(id: &str, id_set: &mut HashSet<String>) -> Result<()> {
    if id.is_empty() {
        return Err(anyhow::anyhow!("ID不能为空"));
    }

    if id.len() > MAX_ID_LENGTH {
        return Err(anyhow::anyhow!(
            "ID '{}' 长度超过限制: {} > {}",
            id,
            id.len(),
            MAX_ID_LENGTH
        ));
    }

    if !id_set.insert(id.to_string()) {
        return Err(anyhow::anyhow!("ID重复: {}", id));
    }

    Ok(())
}

/// 验证坐标边界
fn validate_bounds(id: &str, bounds: &CoordinateBounds) -> Result<()> {
    if !bounds.is_within_screen() {
        return Err(anyhow::anyhow!(
            "元素 '{}' 超出屏幕范围: ({}, {}, {}, {})",
            id,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height
        ));
    }
    Ok(())
}

/// 验证图标ID
fn validate_icon_id(icon_id: &str) -> Result<()> {
    // 这里应该从生成的图标文件中获取有效ID列表
    // 为了简化，这里只做基本验证
    if icon_id.is_empty() {
        return Err(anyhow::anyhow!("图标ID不能为空"));
    }

    // 可以添加更多验证逻辑
    Ok(())
}

/// 验证条件表达式
fn validate_condition(condition: &str) -> Result<()> {
    if condition.len() > MAX_CONDITION_LENGTH {
        return Err(anyhow::anyhow!(
            "条件表达式长度超过限制: {} > {}",
            condition.len(),
            MAX_CONDITION_LENGTH
        ));
    }

    // 提取变量名
    let variables = extract_variables_from_condition(condition);

    for var in variables {
        if !is_valid_system_state_variable(&var) {
            return Err(anyhow::anyhow!(
                "条件表达式 '{}' 中的变量 '{}' 不存在于SystemState",
                condition,
                var
            ));
        }
    }

    Ok(())
}

/// 从条件表达式中提取变量
fn extract_variables_from_condition(condition: &str) -> Vec<String> {
    let mut variables = Vec::new();
    let mut chars = condition.chars().collect::<Vec<_>>();
    let mut i = 0;

    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
            let start = i + 2;
            let mut end = start;

            while end < chars.len()
                && !(chars[end] == '}' && end + 1 < chars.len() && chars[end + 1] == '}')
            {
                end += 1;
            }

            if end < chars.len() {
                let var_str: String = chars[start..end].iter().collect();
                let var = var_str.trim().to_string();
                if !var.is_empty() {
                    variables.push(var);
                }
                i = end + 2;
                continue;
            }
        }
        i += 1;
    }

    variables
}

/// 检查变量是否为SystemState中的有效字段
fn is_valid_system_state_variable(variable: &str) -> bool {
    VALID_SYSTEM_STATE_FIELDS.contains(&variable)
}

// ==================== 导出函数 ====================

/// 构建布局数据（对外接口）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    LayoutBuilder::build(config, progress)
}
