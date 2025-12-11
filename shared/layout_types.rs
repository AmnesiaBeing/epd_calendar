//! 编译期+运行时共用的布局类型定义
//! 包含所有布局节点的基础结构、常量约束、核心trait
//! 适配嵌入式环境：移除HashMap，改用顺序表实现ID映射

use serde::{Deserialize, Serialize};

use crate::shared::generated_font_size::FontSize;

// ==================== 核心常量（对齐规则） ====================
pub const SCREEN_WIDTH: u16 = 800;
pub const SCREEN_HEIGHT: u16 = 480;

// 长度/数量约束
pub const MAX_ID_LENGTH: usize = 64; // ID最大长度
pub const MAX_CONTENT_LENGTH: usize = 128; // 文本内容最大长度
pub const MAX_CONDITION_LENGTH: usize = 128; // 条件字符串最大长度
pub const MAX_CHILDREN_COUNT: usize = 20; // 容器子节点最大数量
pub const MAX_NEST_LEVEL: usize = 10; // 节点最大嵌套层级

// 属性范围约束
pub const MIN_THICKNESS: u16 = 1; // 线条/描边最小宽度
pub const MAX_THICKNESS: u16 = 3; // 线条/描边最大宽度
pub const MIN_MAX_LINES: u8 = 1; // 文本最小行数
pub const MAX_MAX_LINES: u8 = 5; // 文本最大行数
pub const MIN_WEIGHT: f32 = 0.0001; // 权重最小值（避免0）
pub const MAX_WEIGHT: f32 = 10.0; // 权重最大值

// ==================== 基础类型 ====================
/// 布局节点ID类型（池化后用u16索引，节省内存）
pub type NodeId = u16;

/// 带长度约束的ID字符串（编译期校验，运行时只读）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdString(pub String);

impl IdString {
    /// 运行时获取字符串（无校验）
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 带长度约束的内容字符串
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentString(pub String);

impl ContentString {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 带长度约束的条件字符串
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionString(pub String);

impl ConditionString {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ==================== 枚举定义 ====================
/// 重要程度（用于颜色映射）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Importance {
    Normal,   // 黑色
    Warning,  // 黄色
    Critical, // 红色
}

/// 文本水平对齐方式
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

/// 容器布局方向
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContainerDirection {
    Horizontal,
    Vertical,
}

// ==================== 子节点布局配置 ====================
/// 子元素布局配置（池化后用NodeId引用，无嵌套）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildLayout {
    /// 引用布局池中的节点ID
    pub node_id: NodeId,
    /// 权重（用于比例布局）
    pub weight: Option<f32>,
    /// 是否绝对定位（忽略父容器布局）
    pub is_absolute: bool,
}

// ==================== 布局节点枚举 ====================
/// 扁平化布局节点（无嵌套，所有子节点用NodeId引用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    Container(Container),
    Text(Text),
    Icon(Icon),
    Line(Line),
    Rectangle(Rectangle),
    Circle(Circle),
}

/// 容器节点（无border属性，符合新规则）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: IdString,
    /// 相对/绝对坐标 [x, y, width, height]
    pub rect: [u16; 4],
    /// 子节点ID列表（池化后无嵌套）
    pub children: Vec<ChildLayout>,
    /// 显示条件（可选）
    pub condition: Option<ConditionString>,
    /// 布局方向
    pub direction: ContainerDirection,
    /// 子节点水平对齐
    pub alignment: TextAlignment,
    /// 子节点垂直对齐
    pub vertical_alignment: VerticalAlignment,
}

/// 文本节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub id: IdString,
    pub rect: [u16; 4],
    pub content: ContentString,
    pub font_size: FontSize,
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    /// 文本最大宽度（自动换行）
    pub max_width: Option<u16>,
    /// 文本最大行数
    pub max_lines: Option<u8>,
}

/// 图标节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub id: IdString,
    pub rect: [u16; 4],
    /// 图标资源ID（支持格式化字符串）
    pub icon_id: IdString,
    pub importance: Option<Importance>,
}

/// 线条节点（符合CSS规范，start/end/thickness）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub id: IdString,
    /// 起点 [x, y]
    pub start: [u16; 2],
    /// 终点 [x, y]
    pub end: [u16; 2],
    /// 线宽（1-3px）
    pub thickness: u16,
    pub importance: Option<Importance>,
}

/// 矩形节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub id: IdString,
    pub rect: [u16; 4],
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    /// 描边宽度（1-3px）
    pub stroke_thickness: u16,
}

/// 圆形节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circle {
    pub id: IdString,
    /// 圆心 [x, y]
    pub center: [u16; 2],
    pub radius: u16,
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

// ==================== 布局池（扁平化存储，嵌入式友好） ====================
/// 布局池（编译期生成，运行时只读）
/// 所有节点扁平化存储，无嵌套，ID映射改用顺序表，适配嵌入式环境
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPool {
    /// 所有布局节点的扁平数组
    pub nodes: Vec<LayoutNode>,
    /// 根节点ID
    pub root_node_id: NodeId,
    /// ID到NodeId的映射
    /// 格式：(节点ID字符串, 节点索引ID)
    pub id_map: Vec<(IdString, NodeId)>,
}

impl LayoutPool {
    /// 运行时通过ID查找节点（顺序表遍历，嵌入式友好）
    /// 节点数量有限（≤100），遍历性能可接受
    pub fn find_node_by_id(&self, id: &str) -> Option<NodeId> {
        self.id_map
            .iter()
            .find(|(existing_id, _)| existing_id.as_str() == id)
            .map(|(_, node_id)| *node_id)
    }

    /// 运行时通过NodeId获取节点
    pub fn get_node(&self, node_id: NodeId) -> Option<&LayoutNode> {
        self.nodes.get(node_id as usize)
    }
}

// ==================== 通用Trait ====================
/// 所有布局元素的通用行为
pub trait Element {
    /// 获取元素ID
    fn id(&self) -> &IdString;
    /// 获取元素的边界矩形（用于碰撞/渲染）
    fn rect(&self) -> [u16; 4];
}

// 实现Element Trait（编译期/运行时共用）
impl Element for LayoutNode {
    fn id(&self) -> &IdString {
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
}

// 辅助函数：线条转边界矩形
pub fn line_to_rect(line: &Line) -> [u16; 4] {
    let x_min = line.start[0].min(line.end[0]);
    let y_min = line.start[1].min(line.end[1]);
    let x_max = line.start[0].max(line.end[0]);
    let y_max = line.start[1].max(line.end[1]);
    [x_min, y_min, x_max - x_min, y_max - y_min]
}

// 辅助函数：圆形转边界矩形
pub fn circle_to_rect(circle: &Circle) -> [u16; 4] {
    let diameter = circle.radius * 2;
    [
        circle.center[0].saturating_sub(circle.radius),
        circle.center[1].saturating_sub(circle.radius),
        diameter,
        diameter,
    ]
}

// 为各节点单独实现Element（可选）
impl Element for Container {
    fn id(&self) -> &IdString {
        &self.id
    }
    fn rect(&self) -> [u16; 4] {
        self.rect
    }
}

impl Element for Text {
    fn id(&self) -> &IdString {
        &self.id
    }
    fn rect(&self) -> [u16; 4] {
        self.rect
    }
}

impl Element for Icon {
    fn id(&self) -> &IdString {
        &self.id
    }
    fn rect(&self) -> [u16; 4] {
        self.rect
    }
}

impl Element for Line {
    fn id(&self) -> &IdString {
        &self.id
    }
    fn rect(&self) -> [u16; 4] {
        line_to_rect(self)
    }
}

impl Element for Rectangle {
    fn id(&self) -> &IdString {
        &self.id
    }
    fn rect(&self) -> [u16; 4] {
        self.rect
    }
}

impl Element for Circle {
    fn id(&self) -> &IdString {
        &self.id
    }
    fn rect(&self) -> [u16; 4] {
        circle_to_rect(self)
    }
}
