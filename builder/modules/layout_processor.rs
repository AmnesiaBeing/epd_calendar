//! 布局处理模块 - 解析YAML布局文件，验证资源合法性，生成二进制布局数据

use crate::builder::config::BuildConfig;
use crate::builder::modules::font_generator::FontSizeConfig;
use crate::builder::utils::progress::ProgressTracker;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

// ==================== 常量定义 ====================

const SCREEN_WIDTH: u16 = 800;
const SCREEN_HEIGHT: u16 = 480;

// ID和字符串长度限制
const MAX_ID_LENGTH: usize = 32;
const MAX_CONTENT_LENGTH: usize = 128;
const MAX_CONDITION_LENGTH: usize = 64;
const MAX_CHILDREN_COUNT: usize = 64;

// ==================== 类型别名 ====================

type String32 = String;
type String128 = String;
type String64 = String;
type String16 = String;

// ==================== 枚举和结构体定义 ====================

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
pub struct CoordinateBounds {
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

/// 布局方向
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LayoutDirection {
    Horizontal, // 水平布局
    Vertical,   // 垂直布局
}

impl FromStr for LayoutDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "horizontal" => Ok(LayoutDirection::Horizontal),
            "vertical" => Ok(LayoutDirection::Vertical),
            _ => Err(format!("无效的布局方向: {}", s)),
        }
    }
}

/// 容器对齐方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContainerAlignment {
    Start,        // 开始对齐（左对齐或上对齐）
    Center,       // 居中对齐
    End,          // 结束对齐（右对齐或下对齐）
    Stretch,      // 拉伸填充
    SpaceBetween, // 两端对齐，子元素之间间隔相等
}

impl FromStr for ContainerAlignment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "start" => Ok(ContainerAlignment::Start),
            "center" => Ok(ContainerAlignment::Center),
            "end" => Ok(ContainerAlignment::End),
            "stretch" => Ok(ContainerAlignment::Stretch),
            "space_between" => Ok(ContainerAlignment::SpaceBetween),
            _ => Err(format!("无效的容器对齐方式: {}", s)),
        }
    }
}

/// 文本水平对齐方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextHorizontalAlignment {
    Left,
    Center,
    Right,
}

impl FromStr for TextHorizontalAlignment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "left" => Ok(TextHorizontalAlignment::Left),
            "center" => Ok(TextHorizontalAlignment::Center),
            "right" => Ok(TextHorizontalAlignment::Right),
            _ => Err(format!("无效的文本水平对齐方式: {}", s)),
        }
    }
}

/// 文本垂直对齐方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextVerticalAlignment {
    Top,
    Middle,
    Bottom,
}

impl FromStr for TextVerticalAlignment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "top" => Ok(TextVerticalAlignment::Top),
            "middle" => Ok(TextVerticalAlignment::Middle),
            "bottom" => Ok(TextVerticalAlignment::Bottom),
            _ => Err(format!("无效的文本垂直对齐方式: {}", s)),
        }
    }
}

// ==================== Element Trait ====================

/// 布局元素Trait，定义所有布局元素必须实现的功能
pub trait Element: std::fmt::Debug {
    /// 获取元素ID
    fn id(&self) -> &str;

    /// 验证元素合法性
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()>;

    /// 验证元素边界是否在屏幕范围内
    fn validate_bounds(&self) -> Result<()>;

    /// 获取元素边界（用于验证）
    fn get_bounds(&self) -> CoordinateBounds;
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

impl Element for LayoutNode {
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

    fn validate_bounds(&self) -> Result<()> {
        match self {
            LayoutNode::Container(c) => c.validate_bounds(),
            LayoutNode::Text(t) => t.validate_bounds(),
            LayoutNode::Icon(i) => i.validate_bounds(),
            LayoutNode::Line(l) => l.validate_bounds(),
            LayoutNode::Rectangle(r) => r.validate_bounds(),
            LayoutNode::Circle(c) => c.validate_bounds(),
        }
    }

    fn get_bounds(&self) -> CoordinateBounds {
        match self {
            LayoutNode::Container(c) => c.get_bounds(),
            LayoutNode::Text(t) => t.get_bounds(),
            LayoutNode::Icon(i) => i.get_bounds(),
            LayoutNode::Line(l) => l.get_bounds(),
            LayoutNode::Rectangle(r) => r.get_bounds(),
            LayoutNode::Circle(c) => c.get_bounds(),
        }
    }
}

/// 容器子元素定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerChild {
    pub node: LayoutNode,
    pub weight: Option<f32>, // 布局权重，None表示使用固定尺寸
}

impl ContainerChild {
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        self.node.validate(id_set)?;

        // 验证weight值
        if let Some(weight) = self.weight {
            if weight <= 0.0 {
                return Err(anyhow::anyhow!(
                    "容器子元素权重必须大于0，当前值: {}",
                    weight
                ));
            }
        }

        Ok(())
    }
}

/// 容器元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String32,
    pub rect: [u16; 4],
    pub direction: LayoutDirection,
    pub main_axis_alignment: ContainerAlignment,
    pub cross_axis_alignment: ContainerAlignment,
    pub children: Vec<ContainerChild>,
    pub condition: Option<String64>,
}

impl Element for Container {
    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        // 验证ID
        validate_id(&self.id, id_set)?;

        // 验证边界
        self.validate_bounds()?;

        // 验证条件表达式
        if let Some(condition) = &self.condition {
            validate_condition(condition)?;
        }

        // 验证子元素数量
        if self.children.len() > MAX_CHILDREN_COUNT {
            return Err(anyhow::anyhow!(
                "容器 '{}' 的子元素数量超过限制: {} > {}",
                self.id,
                self.children.len(),
                MAX_CHILDREN_COUNT
            ));
        }

        // 验证子元素
        for (index, child) in self.children.iter().enumerate() {
            child
                .validate(id_set)
                .with_context(|| format!("容器 '{}' 的第{}个子元素验证失败", self.id, index + 1))?;
        }

        // 验证布局权重
        let has_weight = self.children.iter().any(|c| c.weight.is_some());
        let has_no_weight = self.children.iter().any(|c| c.weight.is_none());

        // 如果混合使用权重和非权重布局，需要验证rect是否设置
        if has_weight && has_no_weight {
            for child in &self.children {
                if child.weight.is_none() {
                    // 对于非权重子元素，检查其是否有有效的rect
                    match &child.node {
                        LayoutNode::Container(c) => {
                            if c.rect[2] == 0 || c.rect[3] == 0 {
                                return Err(anyhow::anyhow!(
                                    "容器 '{}' 中非权重子容器必须设置宽度和高度",
                                    self.id
                                ));
                            }
                        }
                        LayoutNode::Text(t) => {
                            if t.size[0] == 0 || t.size[1] == 0 {
                                return Err(anyhow::anyhow!(
                                    "容器 '{}' 中非权重文本元素必须设置宽度和高度",
                                    self.id
                                ));
                            }
                        }
                        LayoutNode::Icon(i) => {
                            // 图标通常有固定尺寸，不需要验证
                        }
                        LayoutNode::Line(_) => {
                            // 线条元素不参与布局
                        }
                        LayoutNode::Rectangle(r) => {
                            if r.rect[2] == 0 || r.rect[3] == 0 {
                                return Err(anyhow::anyhow!(
                                    "容器 '{}' 中非权重矩形元素必须设置宽度和高度",
                                    self.id
                                ));
                            }
                        }
                        LayoutNode::Circle(c) => {
                            if c.radius == 0 {
                                return Err(anyhow::anyhow!(
                                    "容器 '{}' 中非权重圆形元素必须设置半径",
                                    self.id
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        let bounds = self.get_bounds();
        validate_bounds(&self.id, &bounds)
    }

    fn get_bounds(&self) -> CoordinateBounds {
        CoordinateBounds::new(self.rect[0], self.rect[1], self.rect[2], self.rect[3])
    }
}

/// 文本元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub id: String32,
    pub position: [u16; 2],
    pub size: [u16; 2],
    pub content: String128,
    pub font_size: FontSizeConfig,
    pub horizontal_alignment: TextHorizontalAlignment,
    pub vertical_alignment: TextVerticalAlignment,
}

impl Element for Text {
    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        // 验证ID
        validate_id(&self.id, id_set)?;

        // 验证边界
        self.validate_bounds()?;

        // 验证内容长度
        if self.content.len() > MAX_CONTENT_LENGTH {
            return Err(anyhow::anyhow!(
                "文本 '{}' 内容长度超过限制: {} > {}",
                self.id,
                self.content.len(),
                MAX_CONTENT_LENGTH
            ));
        }

        // 验证内容是否为空
        if self.content.is_empty() {
            return Err(anyhow::anyhow!("文本元素 '{}' 内容不能为空", self.id));
        }

        // 验证占位符格式（只验证花括号匹配，不验证字段名）
        if self.content.contains("{{") || self.content.contains("}}") {
            validate_placeholder_format(&self.content)?;
        }

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        let bounds = self.get_bounds();
        validate_bounds(&self.id, &bounds)
    }

    fn get_bounds(&self) -> CoordinateBounds {
        CoordinateBounds::new(
            self.position[0],
            self.position[1],
            self.size[0],
            self.size[1],
        )
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

impl Element for Icon {
    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        // 验证ID
        validate_id(&self.id, id_set)?;

        // 验证边界
        self.validate_bounds()?;

        // 验证图标ID
        validate_icon_id(&self.icon_id)?;

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        let bounds = self.get_bounds();
        validate_bounds(&self.id, &bounds)
    }

    fn get_bounds(&self) -> CoordinateBounds {
        // 图标通常有固定尺寸，这里假设为0x0，实际渲染时使用图标资源尺寸
        CoordinateBounds::new(self.position[0], self.position[1], 0, 0)
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

impl Element for Line {
    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        // 验证ID
        validate_id(&self.id, id_set)?;

        // 验证边界
        self.validate_bounds()?;

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        // 验证起点和终点都在屏幕内
        let start_bounds = CoordinateBounds::new(self.start[0], self.start[1], 0, 0);
        let end_bounds = CoordinateBounds::new(self.end[0], self.end[1], 0, 0);

        validate_bounds(&self.id, &start_bounds)?;
        validate_bounds(&self.id, &end_bounds)
    }

    fn get_bounds(&self) -> CoordinateBounds {
        let x_min = self.start[0].min(self.end[0]);
        let y_min = self.start[1].min(self.end[1]);
        let width = self.start[0].abs_diff(self.end[0]);
        let height = self.start[1].abs_diff(self.end[1]);

        CoordinateBounds::new(x_min, y_min, width, height)
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

impl Element for Rectangle {
    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        // 验证ID
        validate_id(&self.id, id_set)?;

        // 验证边界
        self.validate_bounds()?;

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        let bounds = self.get_bounds();
        validate_bounds(&self.id, &bounds)
    }

    fn get_bounds(&self) -> CoordinateBounds {
        CoordinateBounds::new(self.rect[0], self.rect[1], self.rect[2], self.rect[3])
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

impl Element for Circle {
    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        // 验证ID
        validate_id(&self.id, id_set)?;

        // 验证边界
        self.validate_bounds()?;

        Ok(())
    }

    fn validate_bounds(&self) -> Result<()> {
        let bounds = self.get_bounds();

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

    fn get_bounds(&self) -> CoordinateBounds {
        let diameter = self.radius * 2;
        CoordinateBounds::new(
            self.center[0].saturating_sub(self.radius),
            self.center[1].saturating_sub(self.radius),
            diameter,
            diameter,
        )
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
    direction: Option<String>,
    main_axis_alignment: Option<String>,
    cross_axis_alignment: Option<String>,
    children: Vec<YamlContainerChild>,
    condition: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YamlContainerChild {
    node: YamlLayoutNode,
    weight: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct YamlText {
    id: String,
    position: Option<[u16; 2]>, // 可选，当在容器中使用权重布局时可省略
    size: Option<[u16; 2]>,     // 可选，当在容器中使用权重布局时可省略
    content: String,
    font_size: String,
    horizontal_alignment: Option<String>,
    vertical_alignment: Option<String>,
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

impl Serialize for FontSizeConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.name)
    }
}

impl<'de> Deserialize<'de> for FontSizeConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let font_size_str = String::deserialize(deserializer)?;
        let build_config = BuildConfig::load()
            .map_err(|e| serde::de::Error::custom(format!("加载构建配置失败: {}", e)))?;

        for font_config in &build_config.font_size_configs {
            if font_config.name.to_lowercase() == font_size_str.to_lowercase() {
                return Ok(font_config.clone());
            }
        }

        let available_sizes: Vec<String> = build_config
            .font_size_configs
            .iter()
            .map(|f| f.name.to_lowercase())
            .collect();

        Err(serde::de::Error::custom(format!(
            "无效的字体尺寸: {}，支持的尺寸: {}",
            font_size_str,
            available_sizes.join(", ")
        )))
    }
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
        let path = &config.main_layout_path;
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
                    children.push(ContainerChild {
                        node: Self::convert_yaml_to_layout(&child.node)?,
                        weight: child.weight,
                    });
                }

                let direction = yaml
                    .direction
                    .as_deref()
                    .map(|d| LayoutDirection::from_str(d))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("布局方向解析失败: {}", e))?
                    .unwrap_or(LayoutDirection::Vertical);

                let main_axis_alignment = yaml
                    .main_axis_alignment
                    .as_deref()
                    .map(|a| ContainerAlignment::from_str(a))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("主轴对齐方式解析失败: {}", e))?
                    .unwrap_or(ContainerAlignment::Start);

                let cross_axis_alignment = yaml
                    .cross_axis_alignment
                    .as_deref()
                    .map(|a| ContainerAlignment::from_str(a))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("交叉轴对齐方式解析失败: {}", e))?
                    .unwrap_or(ContainerAlignment::Start);

                Ok(LayoutNode::Container(Container {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "容器ID")?,
                    rect: yaml.rect.unwrap_or([0, 0, 0, 0]),
                    direction,
                    main_axis_alignment,
                    cross_axis_alignment,
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
                let font_size = FontSizeConfig::deserialize(
                    serde::de::value::StringDeserializer::new(yaml.font_size.clone()),
                )
                .map_err(|e: serde::de::value::Error| anyhow::anyhow!("字体尺寸解析失败: {}", e))?;

                let horizontal_alignment = yaml
                    .horizontal_alignment
                    .as_deref()
                    .map(|a| TextHorizontalAlignment::from_str(a))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本水平对齐解析失败: {}", e))?
                    .unwrap_or(TextHorizontalAlignment::Left);

                let vertical_alignment = yaml
                    .vertical_alignment
                    .as_deref()
                    .map(|a| TextVerticalAlignment::from_str(a))
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("文本垂直对齐解析失败: {}", e))?
                    .unwrap_or(TextVerticalAlignment::Top);

                Ok(LayoutNode::Text(Text {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "文本ID")?,
                    position: yaml.position.unwrap_or([0, 0]),
                    size: yaml.size.unwrap_or([0, 0]),
                    content: Self::validate_and_truncate_string(
                        &yaml.content,
                        MAX_CONTENT_LENGTH,
                        "文本内容",
                    )?,
                    font_size,
                    horizontal_alignment,
                    vertical_alignment,
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
        let output_dir = &config.output_dir;
        fs::create_dir_all(output_dir)
            .with_context(|| format!("创建输出目录失败: {}", output_dir.display()))?;

        let bin_path = output_dir.join("generated_layouts.bin");
        fs::write(&bin_path, layout_bin)
            .with_context(|| format!("写入二进制文件失败: {}", bin_path.display()))?;

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
//! 不要手动修改此文件

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
    if icon_id.is_empty() {
        return Err(anyhow::anyhow!("图标ID不能为空"));
    }
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

    // 只验证基本的格式，不验证字段名（运行时处理）
    let variables = extract_variables_from_condition(condition);
    for var in variables {
        if var.trim().is_empty() {
            return Err(anyhow::anyhow!("条件表达式中包含空的占位符变量"));
        }
    }

    Ok(())
}

/// 验证占位符格式（只验证花括号匹配，不验证字段名）
fn validate_placeholder_format(content: &str) -> Result<()> {
    let mut stack = Vec::new();
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next(); // 跳过第二个 '{'
                stack.push('{');
            }
        } else if c == '}' {
            if chars.peek() == Some(&'}') {
                chars.next(); // 跳过第二个 '}'
                if stack.pop().is_none() {
                    return Err(anyhow::anyhow!("占位符花括号不匹配: 多余的 '}}'"));
                }
            }
        }
    }

    if !stack.is_empty() {
        return Err(anyhow::anyhow!("占位符花括号不匹配: 缺少 '}}'"));
    }

    Ok(())
}

/// 从条件表达式中提取变量
fn extract_variables_from_condition(condition: &str) -> Vec<String> {
    let mut variables = Vec::new();
    let chars = condition.chars().collect::<Vec<_>>();
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

// ==================== 导出函数 ====================

/// 构建布局数据（对外接口）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    LayoutBuilder::build(config, progress)
}
