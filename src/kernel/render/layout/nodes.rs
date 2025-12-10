//! 布局节点定义
//! 定义所有布局节点的数据结构，与 builder/modules/layout_processor.rs 中的结构保持一致

use alloc::boxed::Box;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

// ==================== 基础类型和常量 ====================

/// ID最大长度
pub const MAX_ID_LENGTH: usize = 32;
/// 内容最大长度
pub const MAX_CONTENT_LENGTH: usize = 128;
/// 条件表达式最大长度
pub const MAX_CONDITION_LENGTH: usize = 128;
/// 最大子节点数量
pub const MAX_CHILDREN_COUNT: usize = 64;

/// 重要程度枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Importance {
    Normal,   // Black
    Warning,  // Yellow
    Critical, // Red
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

/// 字体大小
pub use crate::assets::generated_fonts::FontSize;

/// 子布局节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildLayout {
    pub node: Box<LayoutNode>,
    pub weight: Option<f32>, // 权重，用于比例布局
    pub is_absolute: bool,   // 是否为绝对定位
}

// ==================== 布局节点枚举 ====================

/// 布局节点类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    Container(Box<Container>),
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

    /// 获取节点矩形区域 [x, y, width, height]
    pub fn rect(&self) -> [u16; 4] {
        match self {
            LayoutNode::Container(container) => container.rect,
            LayoutNode::Text(text) => text.rect,
            LayoutNode::Icon(icon) => icon.rect,
            LayoutNode::Line(line) => line_to_rect(line),
            LayoutNode::Rectangle(rect) => rect.rect,
            LayoutNode::Circle(circle) => circle_to_rect(circle),
        }
    }
}

// ==================== 具体节点类型 ====================

/// 容器节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: heapless::String<MAX_ID_LENGTH>,
    pub rect: [u16; 4],
    pub children: heapless::Vec<ChildLayout, MAX_CHILDREN_COUNT>,
    pub condition: Option<heapless::String<MAX_CONDITION_LENGTH>>,
    pub direction: ContainerDirection,
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub border: Border,
}

/// 文本节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub id: heapless::String<MAX_ID_LENGTH>,
    pub rect: [u16; 4],
    pub content: heapless::String<MAX_CONTENT_LENGTH>,
    pub font_size: FontSize,
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub max_width: Option<u16>, // 文本最大宽度，用于自动换行
    pub max_lines: Option<u8>,  // 文本最大行数
}

/// 图标节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub id: heapless::String<MAX_ID_LENGTH>,
    pub rect: [u16; 4],
    pub icon_id: heapless::String<MAX_ID_LENGTH>, // 可以是静态ID，也可以是格式化字符串
    pub importance: Option<Importance>,
}

/// 线条节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub id: heapless::String<MAX_ID_LENGTH>,
    pub start: [u16; 2],
    pub end: [u16; 2],
    pub thickness: u16,
    pub importance: Option<Importance>,
}

/// 矩形节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub id: heapless::String<MAX_ID_LENGTH>,
    pub rect: [u16; 4],
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

/// 圆形节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circle {
    pub id: heapless::String<MAX_ID_LENGTH>,
    pub center: [u16; 2],
    pub radius: u16,
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

// ==================== 辅助函数 ====================

/// 将线条转换为矩形区域
fn line_to_rect(line: &Line) -> [u16; 4] {
    let x1 = line.start[0];
    let y1 = line.start[1];
    let x2 = line.end[0];
    let y2 = line.end[1];

    let min_x = x1.min(x2);
    let min_y = y1.min(y2);
    let max_x = x1.max(x2);
    let max_y = y1.max(y2);

    let half_thickness = line.thickness / 2;

    [
        min_x.saturating_sub(half_thickness),
        min_y.saturating_sub(half_thickness),
        max_x
            .saturating_add(half_thickness)
            .saturating_sub(min_x.saturating_sub(half_thickness)),
        max_y
            .saturating_add(half_thickness)
            .saturating_sub(min_y.saturating_sub(half_thickness)),
    ]
}

/// 将圆形转换为矩形区域
fn circle_to_rect(circle: &Circle) -> [u16; 4] {
    let diameter = (circle.radius * 2).saturating_add(circle.stroke_thickness);
    [
        circle.center[0].saturating_sub(circle.radius.saturating_add(circle.stroke_thickness / 2)),
        circle.center[1].saturating_sub(circle.radius.saturating_add(circle.stroke_thickness / 2)),
        diameter,
        diameter,
    ]
}
