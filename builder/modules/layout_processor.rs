//! 布局处理模块 - 解析YAML布局文件，验证资源合法性，生成二进制布局数据

#![allow(unused)]

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

/// 边框定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Border {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Default for Border {
    fn default() -> Self {
        Self {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        }
    }
}

impl Border {
    /// 验证边框是否在合理范围内
    fn validate(&self, container_bounds: &CoordinateBounds) -> Result<()> {
        if self.top > container_bounds.height
            || self.bottom > container_bounds.height
            || self.left > container_bounds.width
            || self.right > container_bounds.width
        {
            return Err(anyhow::anyhow!("边框宽度超出容器范围"));
        }
        Ok(())
    }
}

/// 文本对齐方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl FromStr for TextAlignment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "left" => Ok(TextAlignment::Left),
            "center" => Ok(TextAlignment::Center),
            "right" => Ok(TextAlignment::Right),
            _ => Err(format!("无效的文本对齐方式: {}", s)),
        }
    }
}

/// 垂直对齐方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

impl FromStr for VerticalAlignment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "top" => Ok(VerticalAlignment::Top),
            "center" => Ok(VerticalAlignment::Center),
            "bottom" => Ok(VerticalAlignment::Bottom),
            _ => Err(format!("无效的垂直对齐方式: {}", s)),
        }
    }
}

/// 容器布局方向
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContainerDirection {
    Horizontal,
    Vertical,
}

impl FromStr for ContainerDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "horizontal" => Ok(ContainerDirection::Horizontal),
            "vertical" => Ok(ContainerDirection::Vertical),
            _ => Err(format!("无效的容器方向: {}", s)),
        }
    }
}

/// 子元素布局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildLayout {
    pub node: LayoutNode,
    pub weight: Option<f32>, // 权重，用于比例布局
    pub is_absolute: bool,   // 是否为绝对定位
}

// ==================== Element trait ====================

/// 元素trait，定义所有布局元素的通用行为
trait Element {
    /// 获取元素ID
    fn id(&self) -> &str;

    /// 获取元素的边界矩形
    fn rect(&self) -> [u16; 4];

    /// 验证元素的基本属性
    fn validate_basic(&self, id_set: &mut HashSet<String>) -> Result<()> {
        validate_id(self.id(), id_set)?;
        Ok(())
    }

    /// 验证元素的位置和大小
    fn validate_bounds(&self) -> Result<()> {
        let rect = self.rect();
        let bounds = CoordinateBounds::new(rect[0], rect[1], rect[2], rect[3]);
        validate_bounds(self.id(), &bounds)
    }

    /// 验证元素的所有属性
    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        self.validate_basic(id_set)?;
        self.validate_bounds()
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

    fn rect(&self) -> [u16; 4] {
        match self {
            LayoutNode::Container(c) => c.rect,
            LayoutNode::Text(t) => t.rect,
            LayoutNode::Icon(i) => i.rect,
            LayoutNode::Line(l) => line_to_rect(l),
            LayoutNode::Rectangle(r) => r.rect,
            LayoutNode::Circle(c) => circle_to_rect(c),
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

/// 将线条转换为边界矩形
fn line_to_rect(line: &Line) -> [u16; 4] {
    let x1 = line.start[0];
    let y1 = line.start[1];
    let x2 = line.end[0];
    let y2 = line.end[1];

    let x_min = x1.min(x2);
    let y_min = y1.min(y2);
    let x_max = x1.max(x2);
    let y_max = y1.max(y2);

    [x_min, y_min, x_max - x_min, y_max - y_min]
}

/// 将圆形转换为边界矩形
fn circle_to_rect(circle: &Circle) -> [u16; 4] {
    let diameter = circle.radius * 2;
    [
        circle.center[0].saturating_sub(circle.radius),
        circle.center[1].saturating_sub(circle.radius),
        diameter,
        diameter,
    ]
}

/// 容器元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String32,
    pub rect: [u16; 4],
    pub children: Vec<ChildLayout>,
    pub condition: Option<String64>,
    pub direction: ContainerDirection,
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub border: Border,
}

impl Element for Container {
    fn id(&self) -> &str {
        &self.id
    }

    fn rect(&self) -> [u16; 4] {
        self.rect
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        self.validate_basic(id_set)?;

        // 验证容器边界
        let bounds = CoordinateBounds::new(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        validate_bounds(&self.id, &bounds)?;

        // 验证条件
        if let Some(condition) = &self.condition {
            validate_condition(condition)?;
        }

        // 验证边框
        self.border.validate(&bounds)?;

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
        let mut child_id_set = HashSet::new();

        for child in &self.children {
            child.node.validate(&mut child_id_set)?;

            // 验证权重不为负数
            if let Some(weight) = child.weight {
                if weight < 0.0 {
                    return Err(anyhow::anyhow!(
                        "容器 '{}' 的子元素 '{}' 权重不能为负数: {}",
                        self.id,
                        child.node.id(),
                        weight
                    ));
                }
            }
        }

        // 验证绝对定位的子元素
        let absolute_children: Vec<_> = self.children.iter().filter(|c| c.is_absolute).collect();

        for absolute_child in absolute_children {
            // 绝对定位的子元素必须在父容器范围内
            let child_rect = absolute_child.node.rect();
            let child_bounds =
                CoordinateBounds::new(child_rect[0], child_rect[1], child_rect[2], child_rect[3]);

            if !is_within_bounds(&child_bounds, &bounds) {
                return Err(anyhow::anyhow!(
                    "绝对定位的元素 '{}' 超出父容器范围",
                    absolute_child.node.id()
                ));
            }
        }

        Ok(())
    }
}

/// 文本元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub id: String32,
    pub rect: [u16; 4],
    pub content: String128,
    pub font_size: FontSizeConfig,
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub max_width: Option<u16>, // 文本最大宽度，用于自动换行
    pub max_lines: Option<u8>,  // 文本最大行数
}

impl Serialize for FontSizeConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // 序列化为字体名称字符串，如 "Small", "Medium", "Large"
        serializer.serialize_str(&self.name)
    }
}

impl<'de> Deserialize<'de> for FontSizeConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // 从字符串反序列化，如 "small", "medium", "large"
        let font_size_str = String::deserialize(deserializer)?;

        // 获取构建配置中的字体尺寸配置
        let build_config = BuildConfig::load()
            .map_err(|e| serde::de::Error::custom(format!("加载构建配置失败: {}", e)))?;

        // 查找匹配的字体配置
        for font_config in &build_config.font_size_configs {
            if font_config.name.to_lowercase() == font_size_str.to_lowercase() {
                return Ok(font_config.clone());
            }
        }

        // 如果没有找到匹配的配置，返回错误
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

impl Element for Text {
    fn id(&self) -> &str {
        &self.id
    }

    fn rect(&self) -> [u16; 4] {
        self.rect
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        self.validate_basic(id_set)?;
        self.validate_bounds()?;

        if self.content.len() > MAX_CONTENT_LENGTH {
            return Err(anyhow::anyhow!(
                "文本 '{}' 内容长度超过限制: {} > {}",
                self.id,
                self.content.len(),
                MAX_CONTENT_LENGTH
            ));
        }

        // 验证内容
        if self.content.is_empty() {
            return Err(anyhow::anyhow!("文本元素 '{}' 内容不能为空", self.id));
        }

        Ok(())
    }
}

/// 图标元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub id: String32,
    pub rect: [u16; 4],
    pub icon_id: String32, // 可以是静态ID，也可以是格式化字符串如{{time.hour}}
    pub importance: Option<Importance>,
}

impl Element for Icon {
    fn id(&self) -> &str {
        &self.id
    }

    fn rect(&self) -> [u16; 4] {
        self.rect
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        self.validate_basic(id_set)?;
        self.validate_bounds()?;

        // 图标ID可以是格式化字符串，运行时解析
        // 这里只验证基本格式
        if self.icon_id.is_empty() {
            return Err(anyhow::anyhow!("图标 '{}' 的icon_id不能为空", self.id));
        }

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

impl Element for Line {
    fn id(&self) -> &str {
        &self.id
    }

    fn rect(&self) -> [u16; 4] {
        line_to_rect(self)
    }

    fn validate_bounds(&self) -> Result<()> {
        // 线条元素特殊处理：起点和终点都在屏幕内即可
        let start_bounds = CoordinateBounds::new(self.start[0], self.start[1], 0, 0);
        let end_bounds = CoordinateBounds::new(self.end[0], self.end[1], 0, 0);

        if !start_bounds.is_within_screen() {
            return Err(anyhow::anyhow!(
                "线条 '{}' 起点超出屏幕范围: ({}, {})",
                self.id,
                self.start[0],
                self.start[1]
            ));
        }

        if !end_bounds.is_within_screen() {
            return Err(anyhow::anyhow!(
                "线条 '{}' 终点超出屏幕范围: ({}, {})",
                self.id,
                self.end[0],
                self.end[1]
            ));
        }

        Ok(())
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        self.validate_basic(id_set)?;
        self.validate_bounds()
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

    fn rect(&self) -> [u16; 4] {
        self.rect
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        self.validate_basic(id_set)?;
        self.validate_bounds()
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

    fn rect(&self) -> [u16; 4] {
        circle_to_rect(self)
    }

    fn validate(&self, id_set: &mut HashSet<String>) -> Result<()> {
        self.validate_basic(id_set)?;

        // 检查圆心
        let center_bounds = CoordinateBounds::new(self.center[0], self.center[1], 0, 0);
        validate_bounds(&self.id, &center_bounds)?;

        // 检查外接矩形
        let bounds = CoordinateBounds::new(
            self.center[0].saturating_sub(self.radius),
            self.center[1].saturating_sub(self.radius),
            self.radius * 2,
            self.radius * 2,
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
    children: Vec<YamlChild>,
    condition: Option<String>,
    direction: Option<String>,
    alignment: Option<String>,
    vertical_alignment: Option<String>,
    border: Option<YamlBorder>,
}

#[derive(Debug, Deserialize)]
struct YamlChild {
    node: YamlLayoutNode,
    weight: Option<f32>,
    is_absolute: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct YamlBorder {
    top: Option<u16>,
    right: Option<u16>,
    bottom: Option<u16>,
    left: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct YamlText {
    id: String,
    rect: [u16; 4],
    content: String,
    font_size: String,
    alignment: Option<String>,
    vertical_alignment: Option<String>,
    max_width: Option<u16>,
    max_lines: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct YamlIcon {
    id: String,
    rect: [u16; 4],
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

        // println!("cargo:warning=  布局处理完成，成功生成布局数据");
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
                    children.push(ChildLayout {
                        node: Self::convert_yaml_to_layout(&child.node)?,
                        weight: child.weight,
                        is_absolute: child.is_absolute.unwrap_or(false),
                    });
                }

                // 解析方向
                let direction = match yaml.direction.as_deref() {
                    Some("vertical") => ContainerDirection::Vertical,
                    Some("horizontal") | None => ContainerDirection::Horizontal,
                    Some(dir) => {
                        return Err(anyhow::anyhow!("无效的容器方向: {}", dir));
                    }
                };

                // 解析对齐方式
                let alignment = match yaml.alignment.as_deref() {
                    Some(align_str) => TextAlignment::from_str(align_str)
                        .map_err(|e| anyhow::anyhow!("容器对齐解析失败: {}", e))?,
                    None => TextAlignment::Left,
                };

                let vertical_alignment = match yaml.vertical_alignment.as_deref() {
                    Some(valign_str) => VerticalAlignment::from_str(valign_str)
                        .map_err(|e| anyhow::anyhow!("容器垂直对齐解析失败: {}", e))?,
                    None => VerticalAlignment::Top,
                };

                // 解析边框
                let border = match &yaml.border {
                    Some(yaml_border) => Border {
                        top: yaml_border.top.unwrap_or(0),
                        right: yaml_border.right.unwrap_or(0),
                        bottom: yaml_border.bottom.unwrap_or(0),
                        left: yaml_border.left.unwrap_or(0),
                    },
                    None => Border::default(),
                };

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
                    direction,
                    alignment,
                    vertical_alignment,
                    border,
                }))
            }

            YamlLayoutNode::Text(yaml) => {
                // 使用FontSizeConfig的Deserialize实现来解析字体尺寸
                let font_size = FontSizeConfig::deserialize(
                    serde::de::value::StringDeserializer::new(yaml.font_size.clone()),
                )
                .map_err(|e: serde::de::value::Error| anyhow::anyhow!("字体尺寸解析失败: {}", e))?;

                let alignment = match yaml.alignment.as_deref() {
                    Some(align_str) => TextAlignment::from_str(align_str)
                        .map_err(|e| anyhow::anyhow!("文本对齐解析失败: {}", e))?,
                    None => TextAlignment::Left,
                };

                let vertical_alignment = match yaml.vertical_alignment.as_deref() {
                    Some(valign_str) => VerticalAlignment::from_str(valign_str)
                        .map_err(|e| anyhow::anyhow!("文本垂直对齐解析失败: {}", e))?,
                    None => VerticalAlignment::Top,
                };

                Ok(LayoutNode::Text(Text {
                    id: Self::validate_and_truncate_string(&yaml.id, MAX_ID_LENGTH, "文本ID")?,
                    rect: yaml.rect,
                    content: Self::validate_and_truncate_string(
                        &yaml.content,
                        MAX_CONTENT_LENGTH,
                        "文本内容",
                    )?,
                    font_size,
                    alignment,
                    vertical_alignment,
                    max_width: yaml.max_width,
                    max_lines: yaml.max_lines,
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
                    rect: yaml.rect,
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
            // println!(
            //     "cargo:warning=警告: {} '{}' 长度超过限制 ({} > {})，已自动截断",
            //     field_name,
            //     s,
            //     s.len(),
            //     max_len
            // );
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

    // 验证宽度和高度是否合理
    if bounds.width == 0 || bounds.height == 0 {
        return Err(anyhow::anyhow!("元素 '{}' 的宽度或高度不能为0", id));
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

    // 不再验证变量是否存在，因为变量在运行时获取
    Ok(())
}

/// 检查一个边界是否在另一个边界内
fn is_within_bounds(inner: &CoordinateBounds, outer: &CoordinateBounds) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right <= outer.right
        && inner.bottom <= outer.bottom
}

// ==================== 导出函数 ====================

/// 构建布局数据（对外接口）
pub fn build(config: &BuildConfig, progress: &ProgressTracker) -> Result<()> {
    LayoutBuilder::build(config, progress)
}
