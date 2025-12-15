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

/// YAML容器节点（适配新布局规则：补充condition/is_absolute等字段）
#[derive(Debug, Deserialize)]
pub struct YamlContainer {
    pub id: String,
    pub position: Option<[u16; 2]>,
    pub anchor: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub children: Vec<YamlChild>,
    pub condition: Option<String>,
    pub direction: Option<String>,
    pub alignment: Option<String>,
    pub vertical_alignment: Option<String>,
    // 新增：容器自身是否绝对定位（规则3.4）
    pub is_absolute: Option<bool>,
}

/// YAML子节点配置（调整weight默认值，规则4.6）
#[derive(Debug, Deserialize)]
pub struct YamlChild {
    pub node: YamlLayoutNode,
    // 权重默认值改为1.0（规则2.1）
    #[serde(default = "default_weight")]
    pub weight: f32,
    #[serde(default)]
    pub is_absolute: bool,
}

/// YAML文本节点（适配新布局规则：补充全量字段）
#[derive(Debug, Deserialize)]
pub struct YamlText {
    pub id: String,
    pub position: Option<[u16; 2]>,
    pub anchor: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub content: String,
    pub font_size: String,
    pub alignment: Option<String>,
    pub vertical_alignment: Option<String>,
    pub max_width: Option<u16>,
    pub max_lines: Option<u8>,
    // 新增：显示条件（规则2.3）
    pub condition: Option<String>,
    // 新增：是否绝对定位（规则3.4）
    #[serde(default)]
    pub is_absolute: bool,
    // 新增：权重（规则4.6）
    #[serde(default = "default_weight")]
    pub weight: f32,
}

/// YAML图标节点（适配新布局规则：补充全量字段）
#[derive(Debug, Deserialize)]
pub struct YamlIcon {
    pub id: String,
    pub position: Option<[u16; 2]>,
    pub anchor: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub icon_id: String,
    pub importance: Option<String>,
    // 新增：显示条件（规则2.3）
    pub condition: Option<String>,
    // 新增：是否绝对定位（规则3.4）
    #[serde(default)]
    pub is_absolute: bool,
    // 新增：对齐方式（规则2.1）
    pub alignment: Option<String>,
    pub vertical_alignment: Option<String>,
    // 新增：权重（规则4.6）
    #[serde(default = "default_weight")]
    pub weight: f32,
}

/// YAML线条节点（适配新布局规则：补充条件和绝对定位）
#[derive(Debug, Deserialize)]
pub struct YamlLine {
    pub id: String,
    pub start: [u16; 2],
    pub end: [u16; 2],
    #[serde(default = "default_thickness")]
    pub thickness: u16,
    pub importance: Option<String>,
    // 新增：显示条件（规则2.3）
    pub condition: Option<String>,
    // 新增：是否绝对定位（规则3.4）
    #[serde(default)]
    pub is_absolute: bool,
}

/// YAML矩形节点（适配新布局规则：补充全量字段）
#[derive(Debug, Deserialize)]
pub struct YamlRectangle {
    pub id: String,
    pub position: Option<[u16; 2]>,
    pub anchor: Option<String>,
    // 矩形宽高改为必填（规则3.5.2）
    pub width: u16,
    pub height: u16,
    pub fill_importance: Option<String>,
    pub stroke_importance: Option<String>,
    #[serde(default = "default_thickness")]
    pub stroke_thickness: u16,
    // 新增：显示条件（规则2.3）
    pub condition: Option<String>,
    // 新增：是否绝对定位（规则3.4）
    #[serde(default)]
    pub is_absolute: bool,
}

/// YAML圆形节点（适配新布局规则：补充全量字段）
#[derive(Debug, Deserialize)]
pub struct YamlCircle {
    pub id: String,
    pub position: Option<[u16; 2]>,
    pub anchor: Option<String>,
    pub radius: u16,
    pub fill_importance: Option<String>,
    pub stroke_importance: Option<String>,
    #[serde(default = "default_thickness")]
    pub stroke_thickness: u16,
    // 新增：显示条件（规则2.3）
    pub condition: Option<String>,
    // 新增：是否绝对定位（规则3.4）
    #[serde(default)]
    pub is_absolute: bool,
}

// ==================== 默认值辅助函数 ====================
fn default_weight() -> f32 {
    1.0 // 权重默认值（规则2.1）
}

fn default_thickness() -> u16 {
    MIN_THICKNESS // 线宽/描边默认值（规则2.1/4.7）
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

impl TryFrom<&str> for TextAlignment {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.trim().to_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "center" | "centre" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(format!("无效的文本对齐: {}", s)),
        }
    }
}

impl TryFrom<&str> for VerticalAlignment {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.trim().to_lowercase().as_str() {
            "top" => Ok(Self::Top),
            "center" | "centre" => Ok(Self::Center),
            "bottom" => Ok(Self::Bottom),
            _ => Err(format!("无效的垂直对齐: {}", s)),
        }
    }
}

impl TryFrom<&str> for ContainerDirection {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.trim().to_lowercase().as_str() {
            "horizontal" | "h" => Ok(Self::Horizontal),
            "vertical" | "v" => Ok(Self::Vertical),
            _ => Err(format!("无效的容器方向: {}", s)),
        }
    }
}

impl TryFrom<&str> for Importance {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.trim().to_lowercase().as_str() {
            "normal" | "black" | "default" => Ok(Self::Normal),
            "warning" | "yellow" | "warn" => Ok(Self::Warning),
            "critical" | "red" | "error" => Ok(Self::Critical),
            _ => Err(format!("无效的重要程度: {}", s)),
        }
    }
}
