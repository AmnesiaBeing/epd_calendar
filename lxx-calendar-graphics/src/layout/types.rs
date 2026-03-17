//! JSON 布局系统类型定义
//!
//! 定义用于描述墨水屏显示布局的数据结构
//! 支持通过 JSON 配置动态定义显示内容和布局

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::Deserialize;

/// 文本对齐方式
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl Default for TextAlign {
    fn default() -> Self {
        Self::Center
    }
}

/// 垂直对齐方式
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

impl Default for VerticalAlign {
    fn default() -> Self {
        Self::Top
    }
}

/// 线条样式
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
    Short,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self::Solid
    }
}

/// 条件判断类型
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Condition {
    /// 字段存在且非空
    Exists,
    /// 等于指定值
    Eq { value: String },
    /// 不等于指定值
    NotEq { value: String },
    /// 大于指定值 (数值比较)
    Gt { value: i32 },
    /// 小于指定值 (数值比较)
    Lt { value: i32 },
    /// 大于等于指定值
    Gte { value: i32 },
    /// 小于等于指定值
    Lte { value: i32 },
}

/// 布局块类型 - 对应不同的渲染元素
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayoutBlock {
    /// 文本块 - 显示单行或多行文本
    Text {
        /// 数据字段名，从数据上下文中获取
        field: String,
        /// 字体大小（像素）
        font_size: u16,
        /// 对齐方式
        #[serde(default)]
        align: TextAlign,
        /// 最大行数，超出部分截断并添加省略号
        max_lines: Option<u16>,
        /// 水平边距（像素），默认使用屏幕宽度的 6%
        margin_x: Option<i16>,
        /// 可选的模板字符串，用于格式化输出
        template: Option<String>,
    },
    /// 图标块
    Icon {
        /// 图标名称
        name: String,
        /// 图标大小（像素）
        size: u16,
    },
    /// 分隔线
    Separator {
        /// 线条样式
        #[serde(default)]
        style: LineStyle,
        /// 线宽（像素）
        line_width: Option<u16>,
        /// 水平边距（像素）
        margin_x: Option<i16>,
        /// 短线长度（仅 Short 样式使用）
        width: Option<u16>,
    },
    /// 间距块 - 添加垂直空白
    Spacer {
        /// 高度（像素）
        height: u16,
    },
    /// 区块 - 包含标题和子块
    Section {
        /// 区块标题
        title: String,
        /// 区块图标（可选）
        icon: Option<String>,
        /// 子布局块
        children: Vec<LayoutBlock>,
    },
    /// 垂直堆叠布局
    VStack {
        /// 子块之间的间距
        spacing: u16,
        /// 子布局块
        children: Vec<LayoutBlock>,
    },
    /// 条件渲染块
    Conditional {
        /// 用于判断的字段名
        field: String,
        /// 条件判断
        condition: Condition,
        /// 条件满足时渲染的子块
        then_children: Vec<LayoutBlock>,
        /// 条件不满足时渲染的子块（可选）
        else_children: Option<Vec<LayoutBlock>>,
    },
    /// 大号数字显示
    BigNumber {
        /// 数据字段名
        field: String,
        /// 字体大小（像素）
        font_size: u16,
        /// 对齐方式
        #[serde(default)]
        align: TextAlign,
        /// 单位后缀（如 "°C", "%"）
        unit: Option<String>,
    },
    /// 进度条
    ProgressBar {
        /// 当前值字段
        field: String,
        /// 最大值字段
        max_field: String,
        /// 进度条宽度（像素）
        width: u16,
        /// 进度条高度（像素）
        height: u16,
        /// 水平边距
        margin_x: Option<i16>,
    },
}

/// 状态栏配置
#[derive(Debug, Clone, Deserialize, Default)]
pub struct StatusBarConfig {
    /// 是否显示日期
    #[serde(default = "default_true")]
    pub show_date: bool,
    /// 是否显示天气
    #[serde(default = "default_true")]
    pub show_weather: bool,
    /// 是否显示电池电量
    #[serde(default = "default_true")]
    pub show_battery: bool,
    /// 是否显示时间
    #[serde(default)]
    pub show_time: bool,
    /// 分隔线线宽
    pub line_width: Option<u16>,
    /// 是否使用虚线
    #[serde(default)]
    pub dashed: bool,
}

/// 页脚配置
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FooterConfig {
    /// 页脚标签
    pub label: String,
    /// 是否显示模式名称
    #[serde(default = "default_true")]
    pub show_mode_name: bool,
    /// 分隔线线宽
    pub line_width: Option<u16>,
    /// 页脚高度
    pub height: Option<u16>,
    /// 是否使用虚线
    #[serde(default)]
    pub dashed: bool,
}

/// 主体布局配置
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BodyConfig {
    /// 布局块列表
    #[serde(default)]
    pub blocks: Vec<LayoutBlock>,
    /// 水平对齐方式
    pub align: Option<TextAlign>,
    /// 垂直对齐方式
    pub vertical_align: Option<VerticalAlign>,
}

/// 完整布局定义
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LayoutDefinition {
    /// 状态栏配置
    pub status_bar: Option<StatusBarConfig>,
    /// 主体内容配置
    #[serde(default)]
    pub body: BodyConfig,
    /// 页脚配置
    pub footer: Option<FooterConfig>,
}

/// 内容配置 - 定义如何获取数据
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentConfig {
    /// 静态内容 - 直接定义字段值
    Static {
        fields: alloc::collections::BTreeMap<String, String>,
    },
    /// 模板内容 - 使用模板字符串
    Template { template: String },
    /// 本地数据源 - 诗词、农历等
    Local { source: LocalSource },
}

/// 本地数据源类型
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalSource {
    /// 每日一言
    Quote,
    /// 诗词
    Poetry,
    /// 农历
    Lunar,
    /// 节气
    SolarTerm,
    /// 日期
    Date,
}

/// 模式定义 - 完整的内容 + 布局配置
#[derive(Debug, Clone, Deserialize)]
pub struct ModeDefinition {
    /// 模式唯一标识符（大写）
    pub mode_id: String,
    /// 显示名称
    pub display_name: String,
    /// 图标名称
    pub icon: Option<String>,
    /// 是否可缓存
    #[serde(default = "default_true")]
    pub cacheable: bool,
    /// 内容配置（可选，设备端通常直接使用内置数据源）
    pub content: Option<ContentConfig>,
    /// 布局定义
    pub layout: LayoutDefinition,
}

fn default_true() -> bool {
    true
}

impl ModeDefinition {
    /// 获取模式 ID（大写）
    pub fn mode_id_upper(&self) -> alloc::string::String {
        self.mode_id.to_uppercase()
    }
}

/// 渲染上下文 - 传递数据和状态
pub struct RenderContext<'a> {
    /// 当前绘制 Y 坐标（从上到下累加）
    pub current_y: u32,
    /// 可用宽度（减去边距）
    pub available_width: u32,
    /// 屏幕宽度
    pub screen_width: u32,
    /// 屏幕高度
    pub screen_height: u32,
    /// 状态栏高度
    pub status_bar_height: u32,
    /// 页脚高度
    pub footer_height: u32,
    /// 数据上下文 - 字段名 -> 值
    pub data: &'a alloc::collections::BTreeMap<alloc::string::String, alloc::string::String>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        screen_width: u32,
        screen_height: u32,
        data: &'a alloc::collections::BTreeMap<alloc::string::String, alloc::string::String>,
    ) -> Self {
        let margin_x = screen_width / 16; // 6.25% 边距
        let status_bar_height = screen_height / 10; // 顶部 10% 给状态栏
        let footer_height = screen_height / 12; // 底部约 8% 给页脚

        Self {
            current_y: status_bar_height,
            available_width: screen_width - margin_x * 2,
            screen_width,
            screen_height,
            status_bar_height,
            footer_height,
            data,
        }
    }

    /// 获取字段值
    pub fn get_field(&self, field: &str) -> Option<&alloc::string::String> {
        self.data.get(field)
    }

    /// 解析模板字符串 - 替换 {field} 为实际值
    pub fn resolve_template(&self, template: &str) -> alloc::string::String {
        let mut result = alloc::string::String::from(template);
        for (key, value) in self.data {
            let placeholder = alloc::format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }

    /// 获取剩余可用高度
    pub fn remaining_height(&self) -> u32 {
        self.screen_height - self.footer_height - self.current_y
    }

    /// 获取默认水平边距
    pub fn default_margin_x(&self) -> u32 {
        self.screen_width / 16
    }
}
