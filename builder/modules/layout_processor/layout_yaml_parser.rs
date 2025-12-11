//! 编译期YAML布局文件解析结构体
//! 仅用于编译期解析，不进入运行时

use super::*;
use serde::Deserialize;

/// YAML布局节点（仅编译期解析用）
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum YamlLayoutNode {
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

/// YAML容器节点（无border属性）
#[derive(Debug, Deserialize)]
pub struct YamlContainer {
    pub id: String,
    pub rect: Option<[u16; 4]>,
    pub children: Vec<YamlChild>,
    pub condition: Option<String>,
    pub direction: Option<String>,
    pub alignment: Option<String>,
    pub vertical_alignment: Option<String>,
}

/// YAML子节点配置
#[derive(Debug, Deserialize)]
pub struct YamlChild {
    pub node: YamlLayoutNode,
    pub weight: Option<f32>,
    pub is_absolute: Option<bool>,
}

/// YAML文本节点
#[derive(Debug, Deserialize)]
pub struct YamlText {
    pub id: String,
    pub rect: [u16; 4],
    pub content: String,
    pub font_size: String,
    pub alignment: Option<String>,
    pub vertical_alignment: Option<String>,
    pub max_width: Option<u16>,
    pub max_lines: Option<u8>,
}

/// YAML图标节点
#[derive(Debug, Deserialize)]
pub struct YamlIcon {
    pub id: String,
    pub rect: [u16; 4],
    pub icon_id: String,
    pub importance: Option<String>,
}

/// YAML线条节点（保留start/end/thickness）
#[derive(Debug, Deserialize)]
pub struct YamlLine {
    pub id: String,
    pub start: [u16; 2],
    pub end: [u16; 2],
    pub thickness: u16,
    pub importance: Option<String>,
}

/// YAML矩形节点
#[derive(Debug, Deserialize)]
pub struct YamlRectangle {
    pub id: String,
    pub rect: [u16; 4],
    pub fill_importance: Option<String>,
    pub stroke_importance: Option<String>,
    pub stroke_thickness: u16,
}

/// YAML圆形节点
#[derive(Debug, Deserialize)]
pub struct YamlCircle {
    pub id: String,
    pub center: [u16; 2],
    pub radius: u16,
    pub fill_importance: Option<String>,
    pub stroke_importance: Option<String>,
    pub stroke_thickness: u16,
}

// YAML枚举解析辅助函数
impl TryFrom<&str> for TextAlignment {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(format!("无效的文本对齐: {}", s)),
        }
    }
}

impl TryFrom<&str> for VerticalAlignment {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "top" => Ok(Self::Top),
            "center" => Ok(Self::Center),
            "bottom" => Ok(Self::Bottom),
            _ => Err(format!("无效的垂直对齐: {}", s)),
        }
    }
}

impl TryFrom<&str> for ContainerDirection {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "horizontal" => Ok(Self::Horizontal),
            "vertical" => Ok(Self::Vertical),
            _ => Err(format!("无效的容器方向: {}", s)),
        }
    }
}

impl TryFrom<&str> for Importance {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "normal" | "black" => Ok(Self::Normal),
            "warning" | "yellow" => Ok(Self::Warning),
            "critical" | "red" => Ok(Self::Critical),
            _ => Err(format!("无效的重要程度: {}", s)),
        }
    }
}
