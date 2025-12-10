//! 布局节点定义

use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

/// 重要程度枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Importance {
    Normal,   // Black
    Warning,  // Yellow
    Critical, // Red
}

/// 文本对齐方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// 垂直对齐方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// 容器方向
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContainerDirection {
    Horizontal,
    Vertical,
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

/// 字体大小配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSizeConfig {
    pub size: u16,
}

/// 子布局定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildLayout {
    pub node: Box<LayoutNode>,
    pub weight: Option<f32>, // 权重，用于比例布局
    pub is_absolute: bool,   // 是否为绝对定位
}

/// 容器节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: heapless::String<32>,
    pub rect: [u16; 4],
    pub children: heapless::Vec<ChildLayout, 64>,
    pub condition: Option<heapless::String<64>>,
    pub direction: ContainerDirection,
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub border: Border,
}

/// 文本节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub id: heapless::String<32>,
    pub rect: [u16; 4],
    pub content: heapless::String<128>,
    pub font_size: FontSizeConfig,
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub max_width: Option<u16>, // 文本最大宽度，用于自动换行
    pub max_lines: Option<u8>,  // 文本最大行数
}

/// 图标节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub id: heapless::String<32>,
    pub rect: [u16; 4],
    pub icon_id: heapless::String<32>, // 可以是静态ID，也可以是格式化字符串
    pub importance: Option<Importance>,
}

/// 线条节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub id: heapless::String<32>,
    pub start: [u16; 2],
    pub end: [u16; 2],
    pub thickness: u16,
    pub importance: Option<Importance>,
}

/// 矩形节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub id: heapless::String<32>,
    pub rect: [u16; 4],
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

/// 圆形节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circle {
    pub id: heapless::String<32>,
    pub center: [u16; 2],
    pub radius: u16,
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

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
    /// 获取节点ID
    pub fn id(&self) -> &str {
        match self {
            LayoutNode::Container(container) => &container.id,
            LayoutNode::Text(text) => &text.id,
            LayoutNode::Icon(icon) => &icon.id,
            LayoutNode::Line(line) => &line.id,
            LayoutNode::Rectangle(rect) => &rect.id,
            LayoutNode::Circle(circle) => &circle.id,
        }
    }

    /// 获取节点矩形区域
    pub fn rect(&self) -> [u16; 4] {
        match self {
            LayoutNode::Container(container) => container.rect,
            LayoutNode::Text(text) => text.rect,
            LayoutNode::Icon(icon) => icon.rect,
            LayoutNode::Line(line) => {
                let x1 = line.start[0];
                let y1 = line.start[1];
                let x2 = line.end[0];
                let y2 = line.end[1];
                let x = x1.min(x2);
                let y = y1.min(y2);
                let width = x2.max(x1) - x + line.thickness;
                let height = y2.max(y1) - y + line.thickness;
                [x, y, width, height]
            },
            LayoutNode::Rectangle(rect) => rect.rect,
            LayoutNode::Circle(circle) => {
                let x = circle.center[0].saturating_sub(circle.radius);
                let y = circle.center[1].saturating_sub(circle.radius);
                let diameter = circle.radius.saturating_mul(2);
                [x, y, diameter, diameter]
            },
        }
    }
}
