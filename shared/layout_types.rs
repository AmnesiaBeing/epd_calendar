// 编译期+运行时共用的布局类型定义
// 适配嵌入式环境：静态不可变，无堆分配，无动态容器

// ==================== 核心常量（对齐规则） ====================
pub const SCREEN_WIDTH: u16 = 800;
pub const SCREEN_HEIGHT: u16 = 480;

// 长度/数量约束
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

// ==================== 枚举定义 ====================
/// 重要程度（用于颜色映射）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Importance {
    Normal,   // 黑色
    Warning,  // 黄色
    Critical, // 红色
}

/// 文本水平对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// 垂直对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// 容器布局方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerDirection {
    Horizontal,
    Vertical,
}

// ==================== 子节点布局配置 ====================
/// 子元素布局配置（池化后用NodeId引用，无嵌套）
#[derive(Debug, Clone, Copy)] // 新增 Copy，适配静态数组
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
#[derive(Debug, Clone)]
pub enum LayoutNode {
    Container(Container),
    Text(Text),
    Icon(Icon),
    Line(Line),
    Rectangle(Rectangle),
    Circle(Circle),
}

/// 容器节点（无border属性，符合新规则）
#[derive(Debug, Clone)]
pub struct Container {
    pub id: Id,
    /// 相对/绝对坐标 [x, y, width, height]
    pub rect: [u16; 4],
    /// 子节点列表（静态数组，适配不可变）
    pub children: ChildLayoutVec,
    /// 显示条件（可选）
    pub condition: Option<Condition>,
    /// 布局方向
    pub direction: ContainerDirection,
    /// 子节点水平对齐
    pub alignment: TextAlignment,
    /// 子节点垂直对齐
    pub vertical_alignment: VerticalAlignment,
}

/// 文本节点
#[derive(Debug, Clone)]
pub struct Text {
    pub id: Id,
    pub rect: [u16; 4],
    pub content: Content,
    pub font_size: FontSize, // 运行时是FontSize,编译期是字符串
    pub alignment: TextAlignment,
    pub vertical_alignment: VerticalAlignment,
    /// 文本最大宽度（自动换行）
    pub max_width: Option<u16>,
    /// 文本最大行数
    pub max_lines: Option<u8>,
}

/// 图标节点
#[derive(Debug, Clone)]
pub struct Icon {
    pub id: Id,
    pub rect: [u16; 4],
    pub icon_id: IconId,
    pub importance: Option<Importance>,
}

/// 线条节点
#[derive(Debug, Clone)]
pub struct Line {
    pub id: Id,
    /// 起点 [x, y]
    pub start: [u16; 2],
    /// 终点 [x, y]
    pub end: [u16; 2],
    /// 线宽（1-3px）
    pub thickness: u16,
    pub importance: Option<Importance>,
}

/// 矩形节点
#[derive(Debug, Clone)]
pub struct Rectangle {
    pub id: Id,
    pub rect: [u16; 4],
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    /// 描边宽度（1-3px）
    pub stroke_thickness: u16,
}

/// 圆形节点
#[derive(Debug, Clone)]
pub struct Circle {
    pub id: Id,
    /// 圆心 [x, y]
    pub center: [u16; 2],
    pub radius: u16,
    pub fill_importance: Option<Importance>,
    pub stroke_importance: Option<Importance>,
    pub stroke_thickness: u16,
}

// ==================== 布局池（扁平化存储，嵌入式友好） ====================
/// 布局池（编译期生成，运行时只读）
#[derive(Debug, Clone)]
pub struct LayoutPool {
    /// 所有布局节点的静态数组（不可变）
    pub nodes: LayoutNodeVec,
    /// 根节点ID
    pub root_node_id: NodeId,
}

#[allow(dead_code)]
impl LayoutPool {
    /// 获取指定节点ID的布局节点引用
    pub fn get_node(&self, node_id: NodeId) -> Option<&LayoutNode> {
        self.nodes.get(node_id as usize)
    }
}
