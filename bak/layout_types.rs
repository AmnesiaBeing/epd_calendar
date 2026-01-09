// 编译期+运行时共用的布局类型定义
// 适配嵌入式环境：静态不可变，无堆分配，无动态容器

// ==================== 核心常量（对齐布局规则） ====================
pub const SCREEN_WIDTH: u16 = 800;
pub const SCREEN_HEIGHT: u16 = 480;

// 长度/数量约束（严格对齐布局规则）
pub const MAX_CHILDREN_COUNT: usize = 10; // 容器子节点最大数量
pub const MAX_ID_LENGTH: usize = 32; // ID最大长度
pub const MAX_CONTENT_LENGTH: usize = 128; // 内容最大长度
pub const MAX_CONDITION_LENGTH: usize = 128; // 条件表达式最大长度
pub const MAX_ICON_ID_LENGTH: usize = 64; // 图标ID最大长度

// ==================== 基础类型 ====================
/// 布局节点ID类型（自增分配，全局唯一，最大值65535）
pub type NodeId = u16;

/// 根节点的父节点ID标识
pub const ROOT_PARENT_ID: NodeId = u16::MAX;

// ==================== 枚举定义（与规则完全一致） ====================
/// 布局模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    Flow,
    Absolute,
}

/// 锚点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// 容器方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Horizontal,
    Vertical,
}

/// 对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
}

// ==================== 布局节点定义（扁平化，无嵌套） ====================

/// 扁平化布局节点（无嵌套，所有子节点用NodeId引用）
#[derive(Debug, Clone)]
pub enum LayoutNode {
    Container(Container),
    Text(Text),
    Icon(Icon),
    Line(Line),
    Rectangle(Rectangle),
}

/// Container元素定义
#[derive(Debug, Clone)]
pub struct Container {
    pub id: NodeId,
    #[cfg(debug_assertions)]
    pub id_str: NodeIdStr, // 仅调试用，Release模式下剔除
    /// 该元素自身位于父元素的布局方式（默认flow）
    pub layout: Layout,
    /// 定位点坐标 [x, y]
    pub position: Option<[u16; 2]>,
    /// 锚点（定位基准点，默认TopLeft）
    pub anchor: Option<Anchor>,
    /// 宽度（可选，自动计算时为None）
    pub width: Option<u16>,
    /// 高度（可选，自动计算时为None）
    pub height: Option<u16>,
    /// 子节点列表（静态数组，适配不可变）
    pub children: ChildLayoutVec,
    /// 显示条件（可选）
    pub condition: Option<Condition>,
    /// 布局方向（默认Horizontal）
    pub direction: Direction,
    /// 子节点水平对齐（默认Left）
    pub alignment: Alignment,
    /// 子节点垂直对齐（默认Top）
    pub vertical_alignment: Alignment,
    /// 节点权重（可选，默认1.0）
    pub weight: Option<f32>,
}

/// 文本节点（适配布局规则：自带大小+可选宽高）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Text {
    pub id: NodeId,
    #[cfg(debug_assertions)]
    pub id_str: NodeIdStr,
    /// 定位点坐标 [x, y]
    pub position: Option<[u16; 2]>,
    /// 宽度（可选，默认按字体+内容计算）
    pub width: Option<u16>,
    pub height: Option<u16>,
    /// 文本内容（必填，支持占位符）
    pub content: Content,
    /// 字体尺寸（默认Medium）
    pub font_size: FontSize,
    /// 水平对齐方式（默认Left）
    pub alignment: Alignment,
    /// 垂直对齐方式（默认Top）
    pub vertical_alignment: Alignment,
    /// 文本最大宽度（默认800）
    pub max_width: Option<u16>,
    /// 文本最大行数（默认1）
    pub max_height: Option<u16>,
    /// 显示条件（可选）
    pub condition: Option<Condition>,
    /// 该元素自身位于父元素的布局方式（默认flow）
    pub layout: Layout,
    /// 权重（用于比例布局，默认1.0）
    pub weight: Option<f32>,
}

/// 图标节点（适配布局规则：自带大小+可选宽高）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Icon {
    pub id: NodeId,
    #[cfg(debug_assertions)]
    pub id_str: NodeIdStr,
    /// 该元素自身位于父元素的布局方式（默认flow）
    pub layout: Layout,
    /// 定位点坐标 [x, y]（默认[0,0]）
    pub position: Option<[u16; 2]>,
    /// 锚点（定位基准点，默认TopLeft）
    pub anchor: Option<Anchor>,
    /// 宽度（可选，默认使用图标原生尺寸）
    pub width: Option<u16>,
    /// 高度（可选，默认使用图标原生尺寸）
    pub height: Option<u16>,
    /// 图标资源ID（必填，格式{模块}:{键}）
    pub icon_id: IconId,
    /// 显示条件（可选）
    pub condition: Option<Condition>,
    /// 水平对齐方式（默认Left）
    pub alignment: Alignment,
    /// 垂直对齐方式（默认Top）
    pub vertical_alignment: Alignment,
    /// 权重（用于比例布局，默认1.0）
    pub weight: Option<f32>,
}

/// 线条节点（保持布局规则：start+end+thickness）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Line {
    pub id: NodeId,
    #[cfg(debug_assertions)]
    pub id_str: NodeIdStr,
    /// 起点坐标 [x, y]（必填）
    pub start: [u16; 2],
    /// 终点坐标 [x, y]（必填）
    pub end: [u16; 2],
    /// 线宽（默认1，范围1-3px）
    pub thickness: u16,
    /// 显示条件（可选）
    pub condition: Option<Condition>,
}

/// 矩形节点（适配布局规则：position+anchor+width+height）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Rectangle {
    pub id: NodeId,
    #[cfg(debug_assertions)]
    pub id_str: NodeIdStr,
    /// 该元素自身位于父元素的布局方式（默认flow）
    pub layout: Layout,
    /// 定位点坐标 [x, y]（默认[0,0]）
    pub position: Option<[u16; 2]>,
    /// 锚点（定位基准点，默认TopLeft）
    pub anchor: Option<Anchor>,
    /// 宽度（必填，≥0）
    pub width: Option<u16>,
    /// 高度（必填，≥0）
    pub height: Option<u16>,
    /// 描边宽度（默认1，范围1-3px）
    pub thickness: u16,
    /// 显示条件（可选）
    pub condition: Option<Condition>,
}

// ==================== 布局池定义 ====================

/// 布局池条目（节点ID，父节点ID，节点数据）
pub type LayoutPoolEntry = (NodeId, NodeId, LayoutNode);

/// 布局池（编译期生成，运行时只读）
#[derive(Debug, Clone, Default)]
pub struct LayoutPool {
    pub nodes: LayoutNodeEntryVec,
    pub root_node_id: NodeId,
}

// ==================== Anchor 辅助实现 ====================
impl Anchor {
    /// 根据锚点计算元素左上角坐标
    pub fn calculate_top_left(&self, position: [u16; 2], width: u16, height: u16) -> [u16; 2] {
        match self {
            Anchor::TopLeft => [position[0], position[1]],
            Anchor::TopCenter => [position[0].saturating_sub(width / 2), position[1]],
            Anchor::TopRight => [position[0].saturating_sub(width), position[1]],
            Anchor::CenterLeft => [position[0], position[1].saturating_sub(height / 2)],
            Anchor::Center => [
                position[0].saturating_sub(width / 2),
                position[1].saturating_sub(height / 2),
            ],
            Anchor::CenterRight => [
                position[0].saturating_sub(width),
                position[1].saturating_sub(height / 2),
            ],
            Anchor::BottomLeft => [position[0], position[1].saturating_sub(height)],
            Anchor::BottomCenter => [
                position[0].saturating_sub(width / 2),
                position[1].saturating_sub(height),
            ],
            Anchor::BottomRight => [
                position[0].saturating_sub(width),
                position[1].saturating_sub(height),
            ],
        }
    }
}
