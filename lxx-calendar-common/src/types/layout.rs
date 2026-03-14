//! 布局定义模块
//! 定义类似 inksight 项目的 JSON 布局格式

extern crate alloc;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

/// 布局定义
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LayoutDefinition {
    /// 状态栏配置
    #[serde(default)]
    pub status_bar: Option<StatusBarConfig>,
    /// 主体内容块列表
    pub body: Vec<Block>,
    /// 页脚配置
    #[serde(default)]
    pub footer: Option<FooterConfig>,
    /// 不同屏幕尺寸的重写配置
    #[serde(default)]
    pub layout_overrides: Option<LayoutOverrides>,
}

/// 状态栏配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusBarConfig {
    /// 线条样式
    #[serde(default)]
    pub style: Option<String>,
    /// 线宽
    #[serde(default)]
    pub line_width: Option<u8>,
    /// 是否虚线
    #[serde(default)]
    pub dashed: Option<bool>,
}

/// 页脚配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FooterConfig {
    /// 页脚标签
    pub label: Option<String>,
    /// 归属模板
    pub attribution_template: Option<String>,
    /// 线宽
    #[serde(default)]
    pub line_width: Option<u8>,
    /// 是否虚线
    #[serde(default)]
    pub dashed: Option<bool>,
    /// 字体
    pub font: Option<String>,
    /// 字体大小
    pub font_size: Option<u16>,
}

/// 不同屏幕尺寸的重写配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LayoutOverrides {
    #[serde(flatten)]
    pub overrides: heapless::String<64>,
}

/// 布局块类型
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Block {
    /// 居中文本
    #[serde(rename = "centered_text")]
    CenteredText(CenteredTextBlock),
    /// 文本块
    #[serde(rename = "text")]
    Text(TextBlock),
    /// 分隔线
    #[serde(rename = "separator")]
    Separator(SeparatorBlock),
    /// 区块
    #[serde(rename = "section")]
    Section(SectionBlock),
    /// 列表
    #[serde(rename = "list")]
    List(ListBlock),
    /// 垂直堆叠
    #[serde(rename = "vertical_stack")]
    VerticalStack(VerticalStackBlock),
    /// 条件块
    #[serde(rename = "conditional")]
    Conditional(ConditionalBlock),
    /// 间距
    #[serde(rename = "spacer")]
    Spacer(SpacerBlock),
    /// 图标 + 文本
    #[serde(rename = "icon_text")]
    IconText(IconTextBlock),
    /// 两列布局
    #[serde(rename = "two_column")]
    TwoColumn(TwoColumnBlock),
    /// 图片
    #[serde(rename = "image")]
    Image(ImageBlock),
    /// 进度条
    #[serde(rename = "progress_bar")]
    ProgressBar(ProgressBarBlock),
    /// 大号数字
    #[serde(rename = "big_number")]
    BigNumber(BigNumberBlock),
    /// 图标列表
    #[serde(rename = "icon_list")]
    IconList(IconListBlock),
    /// 键值对
    #[serde(rename = "key_value")]
    KeyValue(KeyValueBlock),
    /// 组
    #[serde(rename = "group")]
    Group(GroupBlock),
    /// 天气图标 + 文本
    #[serde(rename = "weather_icon_text")]
    WeatherIconText(WeatherIconTextBlock),
    /// 预报卡片
    #[serde(rename = "forecast_cards")]
    ForecastCards(ForecastCardsBlock),
}

/// 居中文本块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CenteredTextBlock {
    /// 内容字段名
    pub field: String,
    /// 字体
    #[serde(default = "default_font")]
    pub font: String,
    /// 字体大小
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    /// 字体文件名（覆盖 font key）
    pub font_name: Option<String>,
    /// 最大宽度比例
    #[serde(default = "default_max_width_ratio")]
    pub max_width_ratio: f32,
    /// 行间距
    #[serde(default = "default_line_spacing")]
    pub line_spacing: u16,
    /// 是否在可用空间中垂直居中
    #[serde(default = "default_true")]
    pub vertical_center: bool,
}

/// 文本块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextBlock {
    /// 内容字段名
    pub field: Option<String>,
    /// 文本模板（如 "{author}"）
    pub template: Option<String>,
    /// 字体
    #[serde(default = "default_font")]
    pub font: String,
    /// 字体大小
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    /// 对齐方式
    #[serde(default)]
    pub align: TextAlign,
    /// X 方向边距
    #[serde(default = "default_margin_x")]
    pub margin_x: u16,
    /// 最大行数
    #[serde(default = "default_max_lines")]
    pub max_lines: u16,
}

/// 分隔线块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SeparatorBlock {
    /// 样式
    #[serde(default)]
    pub style: SeparatorStyle,
    /// X 方向边距
    #[serde(default = "default_margin_x")]
    pub margin_x: u16,
    /// 固定宽度（用于短分隔线）
    pub width: Option<u16>,
    /// 线宽
    #[serde(default = "default_line_width")]
    pub line_width: u8,
}

/// 区块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SectionBlock {
    /// 标题
    pub title: String,
    /// 图标
    pub icon: Option<String>,
    /// 标题字体
    #[serde(default = "default_font")]
    pub title_font: String,
    /// 标题字体大小
    #[serde(default = "default_title_font_size")]
    pub title_font_size: u16,
    /// 子块
    pub children: Vec<Block>,
}

/// 列表块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListBlock {
    /// 内容字段名（必须是数组）
    pub field: String,
    /// 最大项目数
    #[serde(default = "default_max_items")]
    pub max_items: u16,
    /// 每项模板（如 "{index}. {name}"）
    pub item_template: Option<String>,
    /// 右对齐的子字段
    pub right_field: Option<String>,
    /// 字体
    #[serde(default = "default_font")]
    pub font: String,
    /// 字体大小
    #[serde(default = "default_list_font_size")]
    pub font_size: u16,
    /// 项目间距
    #[serde(default = "default_item_spacing")]
    pub item_spacing: u16,
    /// X 方向边距
    #[serde(default = "default_margin_x")]
    pub margin_x: u16,
    /// 是否编号
    #[serde(default)]
    pub numbered: bool,
}

/// 垂直堆叠块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerticalStackBlock {
    /// 子块
    pub children: Vec<Block>,
    /// 间距
    #[serde(default)]
    pub spacing: u16,
}

/// 条件块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConditionalBlock {
    /// 要评估的字段
    pub field: String,
    /// 条件列表
    pub conditions: Vec<Condition>,
    /// 无匹配时的回退块
    pub fallback_children: Option<Vec<Block>>,
}

/// 条件
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Condition {
    /// 操作符
    pub op: ConditionOp,
    /// 比较值
    pub value: Option<serde_json::Value>,
    /// 子块
    pub children: Vec<Block>,
}

/// 条件操作符
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOp {
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
    LenEq,
    LenGt,
    Exists,
}

/// 间距块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpacerBlock {
    /// 高度
    #[serde(default = "default_spacer_height")]
    pub height: u16,
}

/// 图标 + 文本块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IconTextBlock {
    /// 图标名
    pub icon: Option<String>,
    /// 静态文本或模板
    pub text: Option<String>,
    /// 内容字段（覆盖 text）
    pub field: Option<String>,
    /// 字体
    #[serde(default = "default_font")]
    pub font: String,
    /// 字体大小
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    /// 图标大小
    #[serde(default = "default_icon_size")]
    pub icon_size: u16,
    /// X 方向边距
    #[serde(default = "default_margin_x")]
    pub margin_x: u16,
}

/// 天气图标 + 文本块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeatherIconTextBlock {
    /// 天气代码字段
    pub code_field: String,
    /// 文本字段
    pub field: String,
    /// 字体大小
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    /// 图标大小
    #[serde(default = "default_icon_size")]
    pub icon_size: u16,
    /// 对齐方式
    #[serde(default)]
    pub align: TextAlign,
    /// X 方向边距
    #[serde(default = "default_margin_x")]
    pub margin_x: u16,
}

/// 两列布局块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TwoColumnBlock {
    /// 左列宽度
    #[serde(default = "default_left_width")]
    pub left_width: u16,
    /// 列间距
    #[serde(default = "default_gap")]
    pub gap: u16,
    /// 左列 X 起始位置
    #[serde(default = "default_left_x")]
    pub left_x: u16,
    /// 左列块
    pub left: Vec<Block>,
    /// 右列块
    pub right: Vec<Block>,
}

/// 图片块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageBlock {
    /// 字段名
    pub field: Option<String>,
    /// X 坐标
    #[serde(default)]
    pub x: u16,
    /// Y 坐标
    #[serde(default)]
    pub y: u16,
    /// 宽度
    pub width: Option<u16>,
    /// 高度
    pub height: Option<u16>,
}

/// 进度条块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProgressBarBlock {
    /// 当前值字段
    pub field: String,
    /// 最大值字段
    pub max_field: String,
    /// 宽度
    #[serde(default = "default_progress_width")]
    pub width: u16,
    /// 高度
    #[serde(default = "default_progress_height")]
    pub height: u16,
    /// X 方向边距
    #[serde(default = "default_margin_x")]
    pub margin_x: u16,
}

/// 大号数字块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BigNumberBlock {
    /// 字段名
    pub field: String,
    /// 字体大小
    #[serde(default = "default_big_number_size")]
    pub font_size: u16,
    /// 对齐方式
    #[serde(default)]
    pub align: TextAlign,
    /// X 方向边距
    #[serde(default)]
    pub margin_x: u16,
    /// 单位
    pub unit: Option<String>,
}

/// 图标列表块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IconListBlock {
    /// 字段名
    pub field: String,
    /// 图标字段
    pub icon_field: Option<String>,
    /// 文本字段
    pub text_field: Option<String>,
    /// 最大项目数
    #[serde(default = "default_max_items")]
    pub max_items: u16,
}

/// 键值对块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyValueBlock {
    /// 字段名
    pub field: String,
    /// 标签
    pub label: Option<String>,
    /// 字体大小
    #[serde(default = "default_font_size")]
    pub font_size: u16,
}

/// 组块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GroupBlock {
    /// 标题
    pub title: String,
    /// 子块
    pub children: Vec<Block>,
}

/// 预报卡片块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForecastCardsBlock {
    /// 字段名（数组）
    pub field: String,
    /// 最大项目数
    #[serde(default = "default_max_items")]
    pub max_items: u16,
    /// X 方向边距
    #[serde(default)]
    pub margin_x: i16,
    /// 卡片间距
    #[serde(default = "default_gap")]
    pub gap: u16,
    /// 图标大小
    #[serde(default = "default_icon_size")]
    pub icon_size: u16,
}

/// 对齐方式
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    #[default]
    Right,
}

/// 分隔线样式
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SeparatorStyle {
    #[default]
    Solid,
    Dashed,
    Short,
}

/// 条件操作符字符串解析
impl ConditionOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "eq" => Some(ConditionOp::Eq),
            "gt" => Some(ConditionOp::Gt),
            "lt" => Some(ConditionOp::Lt),
            "gte" => Some(ConditionOp::Gte),
            "lte" => Some(ConditionOp::Lte),
            "len_eq" => Some(ConditionOp::LenEq),
            "len_gt" => Some(ConditionOp::LenGt),
            "exists" => Some(ConditionOp::Exists),
            _ => None,
        }
    }
}

// 默认值函数
fn default_font() -> String {
    "noto_serif_regular".to_string()
}

fn default_font_size() -> u16 {
    14
}

fn default_title_font_size() -> u16 {
    14
}

fn default_list_font_size() -> u16 {
    13
}

fn default_icon_size() -> u16 {
    12
}

fn default_big_number_size() -> u16 {
    42
}

fn default_max_width_ratio() -> f32 {
    0.88
}

fn default_line_spacing() -> u16 {
    8
}

fn default_margin_x() -> u16 {
    24
}

fn default_left_x() -> u16 {
    15
}

fn default_max_lines() -> u16 {
    3
}

fn default_max_items() -> u16 {
    8
}

fn default_item_spacing() -> u16 {
    16
}

fn default_spacer_height() -> u16 {
    12
}

fn default_left_width() -> u16 {
    120
}

fn default_gap() -> u16 {
    8
}

fn default_progress_width() -> u16 {
    80
}

fn default_progress_height() -> u16 {
    6
}

fn default_line_width() -> u8 {
    1
}

fn default_true() -> bool {
    true
}
