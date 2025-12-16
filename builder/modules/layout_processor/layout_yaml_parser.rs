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
}

/// YAML容器节点
#[derive(Debug, Deserialize)]
pub struct YamlContainer {
    pub id: String,
    #[serde(default)]
    pub layout: String, // "flow" 或 "absolute"
    pub position: Option<[i16; 2]>,
    pub anchor: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    #[serde(default)]
    pub children: Vec<YamlChild>,
    pub condition: Option<String>,
    #[serde(default)]
    pub direction: String, // "horizontal" 或 "vertical"
    pub alignment: Option<String>,
    pub vertical_alignment: Option<String>,
    pub weight: Option<f32>,
}

/// YAML子节点配置（简化，weight移到节点内部）
#[derive(Debug, Deserialize)]
pub struct YamlChild {
    pub node: YamlLayoutNode,
}

/// YAML文本节点
#[derive(Debug, Deserialize)]
pub struct YamlText {
    pub id: String,
    #[serde(default)]
    pub layout: String, // "flow" 或 "absolute"
    pub position: Option<[i16; 2]>,
    pub content: String,
    pub font_size: String,
    pub alignment: Option<String>,
    pub vertical_alignment: Option<String>,
    pub max_width: Option<u16>,
    pub max_height: Option<u16>,
    pub weight: Option<f32>,
    pub condition: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
}

/// YAML图标节点
#[derive(Debug, Deserialize)]
pub struct YamlIcon {
    pub id: String,
    #[serde(default)]
    pub layout: String,
    pub position: Option<[i16; 2]>,
    pub anchor: Option<String>,
    pub icon_id: String,
    pub alignment: Option<String>,
    pub vertical_alignment: Option<String>,
    pub weight: Option<f32>,
    pub condition: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
}

/// YAML线条节点
#[derive(Debug, Deserialize)]
pub struct YamlLine {
    pub id: String,
    #[serde(default = "default_thickness")]
    pub thickness: u16,
    pub condition: Option<String>,
    pub start: [i16; 2],
    pub end: [i16; 2],
}

/// YAML矩形节点
#[derive(Debug, Deserialize)]
pub struct YamlRectangle {
    pub id: String,
    #[serde(default)]
    pub layout: String, // "flow" 或 "absolute"
    pub position: Option<[i16; 2]>,
    pub anchor: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    #[serde(default = "default_thickness")]
    pub thickness: u16,
    pub condition: Option<String>,
}

// ==================== 默认值辅助函数 ====================
fn default_thickness() -> u16 {
    1 // 线宽/描边默认值
}

// ==================== 枚举解析辅助函数（增强容错） ====================

impl TryFrom<&str> for Anchor {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.trim().to_lowercase().replace('_', "-").as_str() {
            "top-left" | "topleft" => Ok(Self::TopLeft),
            "top-center" | "topcenter" => Ok(Self::TopCenter),
            "top-right" | "topright" => Ok(Self::TopRight),
            "center-left" | "centerleft" => Ok(Self::CenterLeft),
            "center" => Ok(Self::Center),
            "center-right" | "centerright" => Ok(Self::CenterRight),
            "bottom-left" | "bottomleft" => Ok(Self::BottomLeft),
            "bottom-center" | "bottomcenter" => Ok(Self::BottomCenter),
            "bottom-right" | "bottomright" => Ok(Self::BottomRight),
            _ => Err(format!("无效的锚点类型: {}", s)),
        }
    }
}

impl TryFrom<&str> for Alignment {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.trim().to_lowercase().as_str() {
            "top" | "left" | "start" => Ok(Self::Start),
            "center" => Ok(Self::Center),
            "bottom" | "right" | "end" => Ok(Self::End),
            _ => Err(format!("无效的垂直对齐: {}", s)),
        }
    }
}

impl TryFrom<&str> for Direction {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.trim().to_lowercase().as_str() {
            "horizontal" | "h" => Ok(Self::Horizontal),
            "vertical" | "v" => Ok(Self::Vertical),
            _ => Err(format!("无效的容器方向: {}", s)),
        }
    }
}
