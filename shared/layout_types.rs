// 编译期+运行时共用的布局类型定义
// 适配嵌入式环境：静态不可变，无堆分配，无动态容器

// ==================== 核心常量（对齐布局规则） ====================
pub const SCREEN_WIDTH: u16 = 800;
pub const SCREEN_HEIGHT: u16 = 480;

// 长度/数量约束（严格对齐布局规则）
pub const MAX_CHILDREN_COUNT: usize = 20; // 容器子节点最大数量（规则3.1.3）
pub const MAX_NEST_LEVEL: usize = 10; // 节点最大嵌套层级（规则3.1.3）
pub const MAX_ID_LENGTH: usize = 32; // ID最大长度（规则1.1）
pub const MAX_CONTENT_LENGTH: usize = 128; // 内容最大长度（规则1.1）
pub const MAX_CONDITION_LENGTH: usize = 128; // 条件表达式最大长度（规则1.1）
pub const MAX_ICON_ID_LENGTH: usize = 64; // 图标ID最大长度（规则1.1）

// 属性范围约束（严格对齐布局规则）
pub const MIN_THICKNESS: u16 = 1; // 线条/描边最小宽度（规则3.5.1/3.5.2）
pub const MAX_THICKNESS: u16 = 3; // 线条/描边最大宽度（规则3.5.1/3.5.2）
pub const MIN_MAX_LINES: u8 = 1; // 文本最小行数（规则2.1）
pub const MAX_MAX_LINES: u8 = 5; // 文本最大行数（扩展约束）
pub const MIN_WEIGHT: f32 = 0.0001; // 权重最小值（避免0，规则4.6）
pub const MAX_WEIGHT: f32 = 10.0; // 权重最大值（规则4.6）
pub const MAX_DIMENSION: u16 = 800; // 宽度/高度最大值（屏幕宽度，规则1.3）
pub const MIN_RADIUS: u16 = 1; // 圆形最小半径（规则3.5.2）
pub const MAX_RADIUS: u16 = 400; // 圆形最大半径（屏幕半宽，规则3.5.2）
pub const DEFAULT_ICON_WIDTH: u16 = 32; // 图标兜底宽度（规则3.1.2）
pub const DEFAULT_ICON_HEIGHT: u16 = 32; // 图标兜底高度（规则3.1.2）

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
    Left, // 默认值（规则2.1）
    Center,
    Right,
}

impl Default for TextAlignment {
    fn default() -> Self {
        TextAlignment::Left
    }
}

/// 垂直对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top, // 默认值（规则2.1）
    Center,
    Bottom,
}

impl Default for VerticalAlignment {
    fn default() -> Self {
        VerticalAlignment::Top
    }
}

/// 容器布局方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerDirection {
    Horizontal, // 默认值（规则2.1）
    Vertical,
}

impl Default for ContainerDirection {
    fn default() -> Self {
        ContainerDirection::Horizontal
    }
}

/// 锚点类型（定位基准点，规则3.3）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,      // 左上角（默认，规则2.1）
    TopCenter,    // 上边缘中点
    TopRight,     // 右上角
    CenterLeft,   // 左边缘中点
    Center,       // 中心
    CenterRight,  // 右边缘中点
    BottomLeft,   // 左下角
    BottomCenter, // 下边缘中点
    BottomRight,  // 右下角
}

impl Default for Anchor {
    /// 默认锚点：左上角（规则2.1）
    fn default() -> Self {
        Self::TopLeft
    }
}

// ==================== 子节点布局配置 ====================
/// 子元素布局配置（池化后用NodeId引用，无嵌套）
#[derive(Debug, Clone, Copy, Default)] // 新增Default，适配静态初始化
pub struct ChildLayout {
    /// 引用布局池中的节点ID
    pub node_id: NodeId,
    /// 权重（用于比例布局，默认1.0，规则2.1/4.6）
    pub weight: f32,
    /// 是否绝对定位（忽略父容器布局，默认false，规则2.1/3.4）
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

/// 容器节点（适配布局规则：position+anchor+width+height，规则2.3/3.2）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Container {
    pub id: Id,
    /// 定位点坐标 [x, y]（默认[0,0]，规则2.1）
    pub position: [u16; 2],
    /// 锚点（定位基准点，默认TopLeft，规则2.1）
    pub anchor: Anchor,
    /// 宽度（可选，自动计算时为None，规则3.2）
    pub width: Option<u16>,
    /// 高度（可选，自动计算时为None，规则3.2）
    pub height: Option<u16>,
    /// 子节点列表（静态数组，适配不可变，规则1.3）
    pub children: ChildLayoutVec,
    /// 显示条件（可选，规则2.3）
    pub condition: Option<Condition>,
    /// 布局方向（默认Horizontal，规则2.1）
    pub direction: ContainerDirection,
    /// 子节点水平对齐（默认Left，规则2.1）
    pub alignment: TextAlignment,
    /// 子节点垂直对齐（默认Top，规则2.1）
    pub vertical_alignment: VerticalAlignment,
}

/// 文本节点（适配布局规则：自带大小+可选宽高，规则2.3/3.1.1）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Text {
    pub id: Id,
    /// 定位点坐标 [x, y]（默认[0,0]，规则2.1）
    pub position: [u16; 2],
    /// 锚点（定位基准点，默认TopLeft，规则2.1）
    pub anchor: Anchor,
    /// 宽度（可选，默认按字体+内容计算，规则3.1.1）
    pub width: Option<u16>,
    /// 高度（可选，默认按字体尺寸计算，规则3.1.1）
    pub height: Option<u16>,
    /// 文本内容（必填，支持占位符，规则2.3/1.1）
    pub content: Content,
    /// 字体尺寸（默认Medium，规则2.1/4.5）
    pub font_size: FontSize,
    /// 水平对齐方式（默认Left，规则2.1）
    pub alignment: TextAlignment,
    /// 垂直对齐方式（默认Top，规则2.1）
    pub vertical_alignment: VerticalAlignment,
    /// 文本最大宽度（默认800，规则2.1）
    pub max_width: Option<u16>,
    /// 文本最大行数（默认1，规则2.1）
    pub max_lines: Option<u8>,
    /// 显示条件（可选，规则2.3）
    pub condition: Option<Condition>,
    /// 是否绝对布局（默认false，规则2.1/3.4）
    pub is_absolute: bool,
    /// 权重（用于比例布局，默认1.0，规则2.1）
    pub weight: f32,
}

/// 图标节点（适配布局规则：自带大小+可选宽高，规则2.3/3.1.2）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Icon {
    pub id: Id,
    /// 定位点坐标 [x, y]（默认[0,0]，规则2.1）
    pub position: [u16; 2],
    /// 锚点（定位基准点，默认TopLeft，规则2.1）
    pub anchor: Anchor,
    /// 宽度（可选，默认使用图标原生尺寸，规则3.1.2）
    pub width: Option<u16>,
    /// 高度（可选，默认使用图标原生尺寸，规则3.1.2）
    pub height: Option<u16>,
    /// 图标资源ID（必填，格式{模块}:{键}，规则2.3/4.4）
    pub icon_id: IconId,
    /// 重要程度（影响颜色）
    pub importance: Option<Importance>,
    /// 显示条件（可选，规则2.3）
    pub condition: Option<Condition>,
    /// 是否绝对布局（默认false，规则2.1/3.4）
    pub is_absolute: bool,
    /// 水平对齐方式（默认Left，规则2.1）
    pub alignment: TextAlignment,
    /// 垂直对齐方式（默认Top，规则2.1）
    pub vertical_alignment: VerticalAlignment,
    /// 权重（用于比例布局，默认1.0，规则2.1）
    pub weight: f32,
}

/// 线条节点（保持布局规则：start+end+thickness，规则2.3/3.5.1）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Line {
    pub id: Id,
    /// 起点坐标 [x, y]（必填，规则2.3）
    pub start: [u16; 2],
    /// 终点坐标 [x, y]（必填，规则2.3）
    pub end: [u16; 2],
    /// 线宽（默认1，范围1-3px，规则2.1/3.5.1/4.7）
    pub thickness: u16,
    /// 重要程度（影响颜色）
    pub importance: Option<Importance>,
    /// 显示条件（可选，规则2.3）
    pub condition: Option<Condition>,
    /// 是否绝对布局（默认false，规则2.1/3.4）
    pub is_absolute: bool,
}

/// 矩形节点（适配布局规则：position+anchor+width+height，规则2.3/3.5.2）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Rectangle {
    pub id: Id,
    /// 定位点坐标 [x, y]（默认[0,0]，规则2.1）
    pub position: [u16; 2],
    /// 锚点（定位基准点，默认TopLeft，规则2.1）
    pub anchor: Anchor,
    /// 宽度（必填，≥0，规则2.3/3.5.2）
    pub width: u16,
    /// 高度（必填，≥0，规则2.3/3.5.2）
    pub height: u16,
    /// 填充重要程度（影响填充颜色）
    pub fill_importance: Option<Importance>,
    /// 描边重要程度（影响描边颜色）
    pub stroke_importance: Option<Importance>,
    /// 描边宽度（默认1，范围1-3px，规则2.1/3.5.2/4.7）
    pub stroke_thickness: u16,
    /// 显示条件（可选，规则2.3）
    pub condition: Option<Condition>,
    /// 是否绝对布局（默认false，规则2.1/3.4）
    pub is_absolute: bool,
}

/// 圆形节点（适配布局规则：position+anchor+radius，规则2.3/3.5.2）
#[derive(Debug, Clone)] // 新增Default，适配静态初始化
pub struct Circle {
    pub id: Id,
    /// 定位点坐标 [x, y]（默认[0,0]，规则2.1）
    pub position: [u16; 2],
    /// 锚点（定位基准点，默认Center，规则2.1/3.5.2）
    pub anchor: Anchor,
    /// 半径（必填，范围1-400px，规则2.3/3.5.2）
    pub radius: u16,
    /// 填充重要程度（影响填充颜色）
    pub fill_importance: Option<Importance>,
    /// 描边重要程度（影响描边颜色）
    pub stroke_importance: Option<Importance>,
    /// 描边宽度（默认1，范围1-3px，规则2.1/3.5.2/4.7）
    pub stroke_thickness: u16,
    /// 显示条件（可选，规则2.3）
    pub condition: Option<Condition>,
    /// 是否绝对布局（默认false，规则2.1/3.4）
    pub is_absolute: bool,
}

// ==================== 布局池（扁平化存储，嵌入式友好） ====================
/// 布局池（编译期生成，运行时只读，规则5）
#[derive(Debug, Clone, Default)]
pub struct LayoutPool {
    /// 所有布局节点的静态数组（不可变，规则5）
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

    /// 校验节点嵌套层级（规则1.3）
    pub fn validate_nest_level(&self, node_id: NodeId, current_level: usize) -> bool {
        if current_level > MAX_NEST_LEVEL {
            return false;
        }
        match self.get_node(node_id) {
            Some(LayoutNode::Container(container)) => container
                .children
                .iter()
                .all(|child| self.validate_nest_level(child.node_id, current_level + 1)),
            _ => true,
        }
    }

    /// 校验容器子节点数量（规则1.3）
    pub fn validate_children_count(&self) -> bool {
        self.nodes.iter().all(|node| match node {
            LayoutNode::Container(container) => container.children.len() <= MAX_CHILDREN_COUNT,
            _ => true,
        })
    }
}

// ==================== Anchor 辅助实现 ====================
impl Anchor {
    /// 根据锚点计算元素左上角坐标（规则3.3）
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

// ==================== 通用Trait（保持原有功能，适配布局规则） ====================
/// 所有布局元素的通用行为
pub trait Element {
    /// 获取元素ID
    fn id(&self) -> Id;
    /// 获取元素的定位点坐标
    fn position(&self) -> [u16; 2];
    /// 获取元素的锚点
    fn anchor(&self) -> Anchor;
    /// 获取元素的边界矩形（用于碰撞/渲染计算）
    fn bounds(&self) -> [u16; 4];
    /// 校验元素属性合法性（规则4）
    fn validate(&self) -> bool;

    /// 根据position+anchor+width+height计算边界矩形 [x, y, width, height]（规则3.3）
    fn calculate_bounds(
        position: [u16; 2],
        anchor: Anchor,
        width: Option<u16>,
        height: Option<u16>,
    ) -> [u16; 4] {
        let w = width.unwrap_or(0);
        let h = height.unwrap_or(0);
        let [x, y] = anchor.calculate_top_left(position, w, h);
        [x, y, w, h]
    }
}

// 实现Element Trait（编译期/运行时共用，适配布局规则）
impl Element for LayoutNode {
    fn id(&self) -> Id {
        match self {
            LayoutNode::Container(c) => c.id.clone(),
            LayoutNode::Text(t) => t.id.clone(),
            LayoutNode::Icon(i) => i.id.clone(),
            LayoutNode::Line(l) => l.id.clone(),
            LayoutNode::Rectangle(r) => r.id.clone(),
            LayoutNode::Circle(c) => c.id.clone(),
        }
    }

    fn position(&self) -> [u16; 2] {
        match self {
            LayoutNode::Container(c) => c.position,
            LayoutNode::Text(t) => t.position,
            LayoutNode::Icon(i) => i.position,
            LayoutNode::Line(l) => l.start, // 线条以起点为定位点
            LayoutNode::Rectangle(r) => r.position,
            LayoutNode::Circle(c) => c.position,
        }
    }

    fn anchor(&self) -> Anchor {
        match self {
            LayoutNode::Container(c) => c.anchor,
            LayoutNode::Text(t) => t.anchor,
            LayoutNode::Icon(i) => i.anchor,
            LayoutNode::Line(_) => Anchor::TopLeft, // 线条默认锚点：起点
            LayoutNode::Rectangle(r) => r.anchor,
            LayoutNode::Circle(c) => c.anchor,
        }
    }

    fn bounds(&self) -> [u16; 4] {
        match self {
            LayoutNode::Container(c) => {
                Self::calculate_bounds(c.position, c.anchor, c.width, c.height)
            }
            LayoutNode::Text(t) => Self::calculate_bounds(t.position, t.anchor, t.width, t.height),
            LayoutNode::Icon(i) => Self::calculate_bounds(i.position, i.anchor, i.width, i.height),
            LayoutNode::Line(l) => line_to_rect(l),
            LayoutNode::Rectangle(r) => {
                Self::calculate_bounds(r.position, r.anchor, Some(r.width), Some(r.height))
            }
            LayoutNode::Circle(c) => circle_to_rect(c),
        }
    }

    fn validate(&self) -> bool {
        match self {
            LayoutNode::Container(c) => c.validate(),
            LayoutNode::Text(t) => t.validate(),
            LayoutNode::Icon(i) => i.validate(),
            LayoutNode::Line(l) => l.validate(),
            LayoutNode::Rectangle(r) => r.validate(),
            LayoutNode::Circle(c) => c.validate(),
        }
    }
}

impl Element for Container {
    fn id(&self) -> Id {
        self.id.clone()
    }

    fn position(&self) -> [u16; 2] {
        self.position
    }

    fn anchor(&self) -> Anchor {
        self.anchor
    }

    fn bounds(&self) -> [u16; 4] {
        Self::calculate_bounds(self.position, self.anchor, self.width, self.height)
    }

    fn validate(&self) -> bool {
        // 校验ID长度（规则1.1）
        if self.id.len() > MAX_ID_LENGTH {
            return false;
        }
        // 校验子节点数量（规则1.3）
        if self.children.len() > MAX_CHILDREN_COUNT {
            return false;
        }
        // 校验坐标范围（规则1.3）
        self.position[0] <= SCREEN_WIDTH && self.position[1] <= SCREEN_HEIGHT
    }
}

impl Element for Text {
    fn id(&self) -> Id {
        self.id.clone()
    }

    fn position(&self) -> [u16; 2] {
        self.position
    }

    fn anchor(&self) -> Anchor {
        self.anchor
    }

    fn bounds(&self) -> [u16; 4] {
        Self::calculate_bounds(self.position, self.anchor, self.width, self.height)
    }

    fn validate(&self) -> bool {
        // 校验ID长度（规则1.1）
        if self.id.len() > MAX_ID_LENGTH {
            return false;
        }
        // 校验内容长度（规则1.1）
        if self.content.len() > MAX_CONTENT_LENGTH {
            return false;
        }
        // 校验条件长度（规则1.1）
        if let Some(cond) = &self.condition {
            if cond.len() > MAX_CONDITION_LENGTH {
                return false;
            }
        }
        // 校验坐标范围（规则1.3）
        let pos_valid = if self.is_absolute {
            self.position[0] <= SCREEN_WIDTH && self.position[1] <= SCREEN_HEIGHT
        } else {
            self.position[0] <= MAX_DIMENSION && self.position[1] <= MAX_DIMENSION
        };
        // 校验最大行数（规则2.1）
        let lines_valid = self.max_lines.map_or(true, |lines| lines >= MIN_MAX_LINES);
        // 校验权重（规则4.6）
        let weight_valid = self.weight >= MIN_WEIGHT && self.weight <= MAX_WEIGHT;

        pos_valid && lines_valid && weight_valid
    }
}

impl Element for Icon {
    fn id(&self) -> Id {
        self.id.clone()
    }

    fn position(&self) -> [u16; 2] {
        self.position
    }

    fn anchor(&self) -> Anchor {
        self.anchor
    }

    fn bounds(&self) -> [u16; 4] {
        Self::calculate_bounds(self.position, self.anchor, self.width, self.height)
    }

    fn validate(&self) -> bool {
        // 校验ID长度（规则1.1）
        if self.id.len() > MAX_ID_LENGTH {
            return false;
        }
        // 校验图标ID长度和格式（规则1.1/4.4）
        if self.icon_id.len() > MAX_ICON_ID_LENGTH || !self.icon_id.contains(':') {
            return false;
        }
        // 校验条件长度（规则1.1）
        if let Some(cond) = &self.condition {
            if cond.len() > MAX_CONDITION_LENGTH {
                return false;
            }
        }
        // 校验坐标范围（规则1.3）
        let pos_valid = if self.is_absolute {
            self.position[0] <= SCREEN_WIDTH && self.position[1] <= SCREEN_HEIGHT
        } else {
            self.position[0] <= MAX_DIMENSION && self.position[1] <= MAX_DIMENSION
        };
        // 校验权重（规则4.6）
        let weight_valid = self.weight >= MIN_WEIGHT && self.weight <= MAX_WEIGHT;

        pos_valid && weight_valid
    }
}

impl Element for Line {
    fn id(&self) -> Id {
        self.id.clone()
    }

    fn position(&self) -> [u16; 2] {
        self.start
    }

    fn anchor(&self) -> Anchor {
        Anchor::TopLeft
    }

    fn bounds(&self) -> [u16; 4] {
        line_to_rect(self)
    }

    fn validate(&self) -> bool {
        // 校验ID长度（规则1.1）
        if self.id.len() > MAX_ID_LENGTH {
            return false;
        }
        // 校验条件长度（规则1.1）
        if let Some(cond) = &self.condition {
            if cond.len() > MAX_CONDITION_LENGTH {
                return false;
            }
        }
        // 校验线宽（规则4.7）
        let thickness_valid = self.thickness >= MIN_THICKNESS && self.thickness <= MAX_THICKNESS;
        // 校验坐标范围（规则1.3）
        let start_valid = if self.is_absolute {
            self.start[0] <= SCREEN_WIDTH && self.start[1] <= SCREEN_HEIGHT
        } else {
            self.start[0] <= MAX_DIMENSION && self.start[1] <= MAX_DIMENSION
        };
        let end_valid = if self.is_absolute {
            self.end[0] <= SCREEN_WIDTH && self.end[1] <= SCREEN_HEIGHT
        } else {
            self.end[0] <= MAX_DIMENSION && self.end[1] <= MAX_DIMENSION
        };

        thickness_valid && start_valid && end_valid
    }
}

impl Element for Rectangle {
    fn id(&self) -> Id {
        self.id.clone()
    }

    fn position(&self) -> [u16; 2] {
        self.position
    }

    fn anchor(&self) -> Anchor {
        self.anchor
    }

    fn bounds(&self) -> [u16; 4] {
        Self::calculate_bounds(
            self.position,
            self.anchor,
            Some(self.width),
            Some(self.height),
        )
    }

    fn validate(&self) -> bool {
        // 校验ID长度（规则1.1）
        if self.id.len() > MAX_ID_LENGTH {
            return false;
        }
        // 校验条件长度（规则1.1）
        if let Some(cond) = &self.condition {
            if cond.len() > MAX_CONDITION_LENGTH {
                return false;
            }
        }
        // 校验描边宽度（规则4.7）
        let stroke_valid =
            self.stroke_thickness >= MIN_THICKNESS && self.stroke_thickness <= MAX_THICKNESS;
        // 校验宽高（规则1.3）
        let size_valid = self.width <= MAX_DIMENSION && self.height <= MAX_DIMENSION;
        // 校验坐标范围（规则1.3）
        let pos_valid = if self.is_absolute {
            self.position[0] <= SCREEN_WIDTH && self.position[1] <= SCREEN_HEIGHT
        } else {
            self.position[0] <= MAX_DIMENSION && self.position[1] <= MAX_DIMENSION
        };

        stroke_valid && size_valid && pos_valid
    }
}

impl Element for Circle {
    fn id(&self) -> Id {
        self.id.clone()
    }

    fn position(&self) -> [u16; 2] {
        self.position
    }

    fn anchor(&self) -> Anchor {
        self.anchor
    }

    fn bounds(&self) -> [u16; 4] {
        circle_to_rect(self)
    }

    fn validate(&self) -> bool {
        // 校验ID长度（规则1.1）
        if self.id.len() > MAX_ID_LENGTH {
            return false;
        }
        // 校验条件长度（规则1.1）
        if let Some(cond) = &self.condition {
            if cond.len() > MAX_CONDITION_LENGTH {
                return false;
            }
        }
        // 校验半径（规则3.5.2）
        let radius_valid = self.radius >= MIN_RADIUS && self.radius <= MAX_RADIUS;
        // 校验描边宽度（规则4.7）
        let stroke_valid =
            self.stroke_thickness >= MIN_THICKNESS && self.stroke_thickness <= MAX_THICKNESS;
        // 校验坐标范围（规则1.3）
        let pos_valid = if self.is_absolute {
            self.position[0] <= SCREEN_WIDTH && self.position[1] <= SCREEN_HEIGHT
        } else {
            self.position[0] <= MAX_DIMENSION && self.position[1] <= MAX_DIMENSION
        };

        radius_valid && stroke_valid && pos_valid
    }
}

/// 线条转边界矩形（规则3.5.1）
pub fn line_to_rect(line: &Line) -> [u16; 4] {
    let x_min = line.start[0].min(line.end[0]);
    let y_min = line.start[1].min(line.end[1]);
    let x_max = line.start[0].max(line.end[0]);
    let y_max = line.start[1].max(line.end[1]);
    [x_min, y_min, x_max - x_min, y_max - y_min]
}

/// 圆形转边界矩形（基于position+anchor+radius，规则3.3/3.5.2）
pub fn circle_to_rect(circle: &Circle) -> [u16; 4] {
    let diameter = circle.radius * 2;
    let [x, y] = circle
        .anchor
        .calculate_top_left(circle.position, diameter, diameter);
    [x, y, diameter, diameter]
}
