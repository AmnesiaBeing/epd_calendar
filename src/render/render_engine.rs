// src/render/render_engine.rs
//! 布局渲染引擎模块
//! 加载编译期生成的布局二进制，反序列化，处理条件过滤和占位符替换，调用文本/图标/图形渲染工具绘制到4色墨水屏

use embedded_graphics::{
    drawable::Drawable,
    geometry::{Point, Size},
    pixelcolor::BinaryColor,
    primitives::{Circle, Line, Rectangle}, // 注意：这里暂时使用BinaryColor，后面需要替换为QuadColor
};
use epd_waveshare::epd7in5_yrd0750ryf665f60::{Display7in5, QuadColor};
use heapless::{String, Vec};
use postcard::from_bytes;

use crate::{
    assets::generated_icons::IconId,            // 生成的图标ID
    assets::generated_layouts::MAIN_LAYOUT_BIN, // 编译期生成的布局二进制
    common::{
        SystemState,
        error::{AppError, Result},
    },
    driver::display::{DefaultDisplayDriver, DisplayDriver},
    render::{
        image_renderer::draw_binary_image, text_renderer::FontSize, text_renderer::TextRenderer,
    },
    tasks::ComponentDataType,
};

// 布局相关结构体定义（与builder中的定义保持一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
enum LayoutNode {
    Container(Container),
    Text(Text),
    Icon(Icon),
    Line(Line),
    Rectangle(Rectangle),
    Circle(Circle),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Container {
    id: String<32>,
    rect: [u16; 4],
    children: Vec<LayoutNode, 64>,
    condition: Option<String<64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Text {
    id: String<32>,
    position: [u16; 2],
    size: [u16; 2],
    content: String<128>,
    font_size: FontSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Icon {
    id: String<32>,
    position: [u16; 2],
    icon_id: String<32>,
    importance: Option<String<16>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Line {
    id: String<32>,
    start: [u16; 2],
    end: [u16; 2],
    thickness: u16,
    importance: Option<String<16>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Rectangle {
    id: String<32>,
    rect: [u16; 4],
    fill_importance: Option<String<16>>,
    stroke_importance: Option<String<16>>,
    stroke_thickness: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Circle {
    id: String<32>,
    center: [u16; 2],
    radius: u16,
    fill_importance: Option<String<16>>,
    stroke_importance: Option<String<16>>,
    stroke_thickness: u16,
}

// 注意：FontSize 已经在 text_renderer.rs 中定义

// 颜色适配墨水屏的 QuadColor
#[derive(Debug, Clone, Copy)]
enum EinkColor {
    Black,
    White,
    Red,
    Yellow,
}

impl EinkColor {
    /// 转换为墨水屏驱动的 QuadColor 类型
    fn to_quad_color(self) -> QuadColor {
        match self {
            EinkColor::Black => QuadColor::Black,
            EinkColor::White => QuadColor::White,
            EinkColor::Red => QuadColor::Red,
            EinkColor::Yellow => QuadColor::Yellow,
        }
    }
}

// 条件表达式解析相关的类型
enum ConditionToken {
    Variable(String<32>),
    Number(f32),
    Boolean(bool),
    Operator(Operator),
    LeftParen,
    RightParen,
}

enum Operator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

impl Operator {
    /// 解析运算符字符串
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "==" => Some(Operator::Equal),
            "!=" => Some(Operator::NotEqual),
            ">=" => Some(Operator::GreaterThanOrEqual),
            "<=" => Some(Operator::LessThanOrEqual),
            ">" => Some(Operator::GreaterThan),
            "<" => Some(Operator::LessThan),
            _ => None,
        }
    }

    /// 执行比较操作
    fn evaluate(&self, left: &Value, right: &Value) -> bool {
        match (self, left, right) {
            (Operator::Equal, Value::Number(a), Value::Number(b)) => a == b,
            (Operator::Equal, Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Operator::NotEqual, Value::Number(a), Value::Number(b)) => a != b,
            (Operator::NotEqual, Value::Boolean(a), Value::Boolean(b)) => a != b,
            (Operator::GreaterThan, Value::Number(a), Value::Number(b)) => a > b,
            (Operator::GreaterThanOrEqual, Value::Number(a), Value::Number(b)) => a >= b,
            (Operator::LessThan, Value::Number(a), Value::Number(b)) => a < b,
            (Operator::LessThanOrEqual, Value::Number(a), Value::Number(b)) => a <= b,
            _ => false, // 类型不匹配时返回false
        }
    }
}

enum Value {
    Number(f32),
    Boolean(bool),
    String(String<64>),
}

// 条件表达式解析器
struct ConditionParser;

impl ConditionParser {
    /// 解析条件表达式字符串
    fn parse(expression: &str) -> Result<Vec<ConditionToken>> {
        let mut tokens = Vec::new();
        let mut chars = expression.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                // 跳过空格
                ' ' => continue,
                // 括号
                '(' => tokens.push(ConditionToken::LeftParen),
                ')' => tokens.push(ConditionToken::RightParen),
                // 运算符
                '=' => {
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(ConditionToken::Operator(Operator::Equal));
                    }
                }
                '!' => {
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(ConditionToken::Operator(Operator::NotEqual));
                    }
                }
                '>' => {
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(ConditionToken::Operator(Operator::GreaterThanOrEqual));
                    } else {
                        tokens.push(ConditionToken::Operator(Operator::GreaterThan));
                    }
                }
                '<' => {
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(ConditionToken::Operator(Operator::LessThanOrEqual));
                    } else {
                        tokens.push(ConditionToken::Operator(Operator::LessThan));
                    }
                }
                // 数字
                '0'..='9' | '.' => {
                    let mut num_str = String::new();
                    num_str
                        .push(c)
                        .map_err(|_| AppError::LayoutConditionParse)?;

                    while let Some(&next_c) = chars.peek() {
                        if next_c.is_ascii_digit() || next_c == '.' {
                            num_str
                                .push(chars.next().unwrap())
                                .map_err(|_| AppError::LayoutConditionParse)?;
                        } else {
                            break;
                        }
                    }

                    let num = num_str
                        .parse::<f32>()
                        .map_err(|_| AppError::LayoutConditionParse)?;
                    tokens.push(ConditionToken::Number(num));
                }
                // 布尔值或变量
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut var_str = String::new();
                    var_str
                        .push(c)
                        .map_err(|_| AppError::LayoutConditionParse)?;

                    while let Some(&next_c) = chars.peek() {
                        if next_c.is_ascii_alphanumeric() || next_c == '_' {
                            var_str
                                .push(chars.next().unwrap())
                                .map_err(|_| AppError::LayoutConditionParse)?;
                        } else {
                            break;
                        }
                    }

                    // 检查是否是布尔值
                    match var_str.as_str() {
                        "true" => tokens.push(ConditionToken::Boolean(true)),
                        "false" => tokens.push(ConditionToken::Boolean(false)),
                        _ => tokens.push(ConditionToken::Variable(var_str)),
                    }
                }
                // 未知字符
                _ => return Err(AppError::LayoutConditionParse),
            }
        }

        Ok(tokens)
    }

    /// 从SystemState获取变量值
    fn get_variable_value(var_name: &str, state: &SystemState) -> Option<Value> {
        match var_name {
            // 电池相关
            "battery_level" => Some(Value::Number(state.battery_level as u8 as f32)),
            "charging" => Some(Value::Boolean(state.charging_status.0)),
            // 网络相关
            "network_connected" => Some(Value::Boolean(state.network_status.0)),
            // 天气相关
            "has_weather" => Some(Value::Boolean(state.weather.daily_forecast.is_some())),
            // 其他变量可以根据需要添加
            _ => None,
        }
    }

    /// 计算表达式的值
    fn evaluate(tokens: &[ConditionToken], state: &SystemState) -> Result<bool> {
        // 简单的表达式求值，只支持二元运算
        // 这里实现一个简单的求值器，假设表达式是 a op b 的形式
        if tokens.len() != 3 {
            return Err(AppError::LayoutConditionParse);
        }

        let left = match &tokens[0] {
            ConditionToken::Variable(var) => Self::get_variable_value(var.as_str(), state)
                .ok_or(AppError::LayoutConditionParse)?,
            ConditionToken::Number(num) => Value::Number(*num),
            ConditionToken::Boolean(b) => Value::Boolean(*b),
            _ => return Err(AppError::LayoutConditionParse),
        };

        let operator = match &tokens[1] {
            ConditionToken::Operator(op) => op,
            _ => return Err(AppError::LayoutConditionParse),
        };

        let right = match &tokens[2] {
            ConditionToken::Variable(var) => Self::get_variable_value(var.as_str(), state)
                .ok_or(AppError::LayoutConditionParse)?,
            ConditionToken::Number(num) => Value::Number(*num),
            ConditionToken::Boolean(b) => Value::Boolean(*b),
            _ => return Err(AppError::LayoutConditionParse),
        };

        Ok(operator.evaluate(&left, &right))
    }

    /// 解析并评估条件表达式
    pub fn parse_and_evaluate(expression: &str, state: &SystemState) -> Result<bool> {
        let tokens = Self::parse(expression)?;
        Self::evaluate(&tokens, state)
    }
}

// 占位符替换逻辑
struct PlaceholderReplacer;

impl PlaceholderReplacer {
    /// 解析并替换文本中的占位符
    pub fn replace_placeholders(text: &str, state: &SystemState) -> Result<String<128>> {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            // 检查是否是占位符的开始
            if c == '{' && chars.peek() == Some(&'{') {
                // 跳过第二个'{'
                chars.next();

                // 解析占位符内容
                let mut placeholder = String::new();
                let mut format_param = String::new();
                let mut has_format_param = false;

                // 读取占位符内容，直到遇到'}'或':'
                while let Some(&next_c) = chars.peek() {
                    if next_c == ':' {
                        has_format_param = true;
                        chars.next();
                        // 读取格式参数
                        while let Some(&format_c) = chars.peek() {
                            if format_c == '}' {
                                break;
                            }
                            format_param
                                .push(chars.next().unwrap())
                                .map_err(|_| AppError::LayoutPlaceholderNotFound)?;
                        }
                        break;
                    } else if next_c == '}' {
                        break;
                    }
                    placeholder
                        .push(chars.next().unwrap())
                        .map_err(|_| AppError::LayoutPlaceholderNotFound)?;
                }

                // 跳过占位符结束的'}}'
                if chars.peek() == Some(&'}') {
                    chars.next();
                    if chars.peek() == Some(&'}') {
                        chars.next();
                    }
                }

                // 去除占位符和格式参数的前后空格
                let placeholder = placeholder.trim();
                let format_param = format_param.trim();

                // 获取占位符对应的值并格式化
                let value = Self::get_placeholder_value(placeholder, format_param, state);
                match value {
                    Ok(formatted) => {
                        // 将格式化后的值添加到结果中
                        result
                            .push_str(&formatted)
                            .map_err(|_| AppError::LayoutPlaceholderNotFound)?;
                    }
                    Err(e) => {
                        // 占位符不存在或格式错误，保留原始占位符
                        result
                            .push_str(&format!(
                                "{{{{{}{}}}}}",
                                placeholder,
                                if has_format_param {
                                    &format!(":{}", format_param)
                                } else {
                                    ""
                                }
                            ))
                            .map_err(|_| AppError::LayoutPlaceholderNotFound)?;
                        log::warn!(
                            "Placeholder '{}' not found or invalid: {:?}",
                            placeholder,
                            e
                        );
                    }
                }
            } else {
                // 普通字符，直接添加到结果中
                result
                    .push(c)
                    .map_err(|_| AppError::LayoutPlaceholderNotFound)?;
            }
        }

        Ok(result)
    }

    /// 获取占位符对应的值
    fn get_placeholder_value(name: &str, format: &str, state: &SystemState) -> Result<String<64>> {
        match name {
            // 时间相关
            "hour" => {
                let hour = state.time.hour;
                if format == "24h" || format == "" {
                    Ok(format!("{:02}", hour).into())
                } else if format == "12h" {
                    let hour_12 = if hour > 12 {
                        hour - 12
                    } else if hour == 0 {
                        12
                    } else {
                        hour
                    };
                    Ok(format!("{:02}", hour_12).into())
                } else {
                    Err(AppError::LayoutPlaceholderNotFound)
                }
            }
            "minute" => Ok(format!("{:02}", state.time.minute).into()),
            "am_pm" => {
                if state.time.hour < 12 {
                    Ok("AM".into())
                } else {
                    Ok("PM".into())
                }
            }
            // 日期相关
            "year" => Ok(format!("{}", state.date.day.year).into()),
            "month" => {
                if format == "number" || format == "" {
                    Ok(format!("{:02}", state.date.day.month).into())
                } else {
                    // 可以添加月份名称的格式化
                    Err(AppError::LayoutPlaceholderNotFound)
                }
            }
            "day" => Ok(format!("{:02}", state.date.day.day).into()),
            "week" => {
                let week_name = state.date.week.name();
                if format == "full" || format == "" {
                    Ok(week_name.into())
                } else if format == "short" {
                    // 取星期的第一个字
                    Ok(week_name.chars().next().unwrap().to_string().into())
                } else {
                    Err(AppError::LayoutPlaceholderNotFound)
                }
            }
            // 天气相关
            "weather_today" => {
                if let Some((_, forecasts)) = &state.weather.daily_forecast {
                    let today = &forecasts[0];
                    Ok(format!("{:?}", today.condition).into())
                } else {
                    Ok("无数据".into())
                }
            }
            "temp_max" => {
                if let Some((_, forecasts)) = &state.weather.daily_forecast {
                    let today = &forecasts[0];
                    Ok(format!("{}", today.max_temp).into())
                } else {
                    Ok("--".into())
                }
            }
            "temp_min" => {
                if let Some((_, forecasts)) = &state.weather.daily_forecast {
                    let today = &forecasts[0];
                    Ok(format!("{}", today.min_temp).into())
                } else {
                    Ok("--".into())
                }
            }
            // 电池相关
            "battery" => {
                let level = state.battery_level as u8 * 20; // 转换为百分比
                Ok(format!("{}", level).into())
            }
            // 名言相关
            "quote_content" => Ok(state.quote.content.into()),
            "quote_author" => Ok(state.quote.author.into()),
            // 其他占位符
            _ => Err(AppError::LayoutPlaceholderNotFound),
        }
    }
}

/// 刷新策略说明
/// 刷新任务(display_task) ← 组件更新(各task)
///        ↓
/// 嵌入式芯片内存缓冲区 (render_engine)
///        ↓
/// 屏幕内部缓冲区 (通过驱动接口传输 render_engine.render_component())
///        ↓
/// 实际显示区域 (调用render_engine.display()时更新)

/// 渲染引擎核心结构体
pub struct RenderEngine {
    /// epd-waveshare的Display结构体（自带缓冲区）
    display: Display7in5,
    /// 显示驱动（已封装硬件细节）
    driver: DefaultDisplayDriver,
    /// 是否处于睡眠状态标志
    is_sleeping: bool,
}

impl RenderEngine {
    pub fn new(driver: DefaultDisplayDriver) -> Result<Self> {
        log::info!("Initializing RenderEngine...");

        // 初始化Display（使用默认配置）
        let display = Display7in5::default();

        log::info!("RenderEngine initialized successfully");

        Ok(Self {
            display,
            driver,
            is_sleeping: false,
        })
    }

    fn init_driver(&mut self) -> Result<()> {
        self.driver.init().map_err(|e| {
            log::error!("Failed to initialize display driver: {}", e);
            AppError::RenderingFailed
        })?;
        Ok(())
    }

    /// 使显示驱动进入睡眠状态
    pub fn sleep_driver(&mut self) -> Result<()> {
        if !self.is_sleeping {
            log::info!("Putting display driver to sleep");
            self.driver.sleep().map_err(|e| {
                log::error!("Failed to sleep display driver: {}", e);
                AppError::RenderingFailed
            })?;
            self.is_sleeping = true;
            log::info!("Display driver is now sleeping");
        }
        Ok(())
    }

    /// 渲染单个组件到内存缓冲区（保留旧的API，用于兼容旧代码）
    pub fn render_component(&mut self, component_data: &ComponentDataType) -> Result<()> {
        // 根据组件类型选择并绘制对应组件
        match component_data {
            ComponentDataType::TimeType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw Time component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::DateType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw Date component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::WeatherType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw Weather component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::QuoteType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw Quote component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::ChargingStatusType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw ChargingStatus component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::BatteryType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw BatteryLevel component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
            ComponentDataType::NetworkStatusType(component) => {
                component.draw(&mut self.display).map_err(|e| {
                    log::error!("Failed to draw NetworkStatus component: {}", e);
                    AppError::RenderingFailed
                })?;
            }
        }

        log::info!("Successfully partially rendered component buffer");

        Ok(())
    }

    /// 加载布局二进制数据
    /// 从generated_layouts.rs读取MAIN_LAYOUT_BIN，使用postcard::from_bytes反序列化为LayoutRoot
    fn load_layout(&self) -> Result<LayoutNode> {
        let layout = postcard::from_bytes(&MAIN_LAYOUT_BIN).map_err(|_| {
            log::error!("Failed to deserialize layout binary");
            AppError::LayoutDeserialize
        })?;
        Ok(layout)
    }

    /// 处理条件过滤
    /// 递归遍历LayoutNode，解析condition表达式，判断是否显示元素
    fn should_show_element(&self, node: &LayoutNode, state: &SystemState) -> bool {
        match node {
            LayoutNode::Container(container) => {
                if let Some(condition) = &container.condition {
                    match ConditionParser::parse_and_evaluate(condition.as_str(), state) {
                        Ok(result) => result,
                        Err(e) => {
                            log::warn!(
                                "Failed to parse condition '{}' for container '{}': {:?}",
                                condition.as_str(),
                                container.id.as_str(),
                                e
                            );
                            // 条件解析失败时默认显示
                            true
                        }
                    }
                } else {
                    // 没有条件时默认显示
                    true
                }
            }
            LayoutNode::Text(text) => {
                if let Some(condition) = &text.condition {
                    match ConditionParser::parse_and_evaluate(condition.as_str(), state) {
                        Ok(result) => result,
                        Err(e) => {
                            log::warn!(
                                "Failed to parse condition '{}' for text '{}': {:?}",
                                condition.as_str(),
                                text.id.as_str(),
                                e
                            );
                            true
                        }
                    }
                } else {
                    true
                }
            }
            LayoutNode::Icon(icon) => {
                if let Some(condition) = &icon.condition {
                    match ConditionParser::parse_and_evaluate(condition.as_str(), state) {
                        Ok(result) => result,
                        Err(e) => {
                            log::warn!(
                                "Failed to parse condition '{}' for icon '{}': {:?}",
                                condition.as_str(),
                                icon.id.as_str(),
                                e
                            );
                            true
                        }
                    }
                } else {
                    true
                }
            }
            LayoutNode::Line(line) => {
                if let Some(condition) = &line.condition {
                    match ConditionParser::parse_and_evaluate(condition.as_str(), state) {
                        Ok(result) => result,
                        Err(e) => {
                            log::warn!(
                                "Failed to parse condition '{}' for line '{}': {:?}",
                                condition.as_str(),
                                line.id.as_str(),
                                e
                            );
                            true
                        }
                    }
                } else {
                    true
                }
            }
            LayoutNode::Rectangle(rect) => {
                if let Some(condition) = &rect.condition {
                    match ConditionParser::parse_and_evaluate(condition.as_str(), state) {
                        Ok(result) => result,
                        Err(e) => {
                            log::warn!(
                                "Failed to parse condition '{}' for rectangle '{}': {:?}",
                                condition.as_str(),
                                rect.id.as_str(),
                                e
                            );
                            true
                        }
                    }
                } else {
                    true
                }
            }
            LayoutNode::Circle(circle) => {
                if let Some(condition) = &circle.condition {
                    match ConditionParser::parse_and_evaluate(condition.as_str(), state) {
                        Ok(result) => result,
                        Err(e) => {
                            log::warn!(
                                "Failed to parse condition '{}' for circle '{}': {:?}",
                                condition.as_str(),
                                circle.id.as_str(),
                                e
                            );
                            true
                        }
                    }
                } else {
                    true
                }
            }
        }
    }

    /// 渲染单个布局节点
    fn render_layout_node(&mut self, node: &LayoutNode, state: &SystemState) {
        // 检查是否应该显示该元素
        if !self.should_show_element(node, state) {
            return;
        }

        match node {
            LayoutNode::Container(container) => {
                // 递归渲染容器的子元素
                for child in &container.children {
                    self.render_layout_node(child, state);
                }
            }
            LayoutNode::Text(text) => {
                // 替换文本中的占位符
                match PlaceholderReplacer::replace_placeholders(text.content.as_str(), state) {
                    Ok(replaced_text) => {
                        // 创建文本渲染器并绘制
                        let text_renderer = TextRenderer {
                            font_size: text.font_size,
                        };

                        // 调用文本渲染器的绘制方法
                        if let Err(e) = text_renderer.draw(
                            &replaced_text,
                            text.position[0],
                            text.position[1],
                            &mut self.display,
                        ) {
                            log::warn!("Failed to render text '{}': {:?}", text.id.as_str(), e);
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to replace placeholders for text '{}': {:?}",
                            text.id.as_str(),
                            e
                        );
                    }
                }
            }
            LayoutNode::Icon(icon) => {
                // 转换icon_id为IconId枚举
                match IconId::from_str(icon.icon_id.as_str()) {
                    Ok(icon_id) => {
                        // 确定图标颜色
                        let color = self.importance_to_color(&icon.importance);

                        // 调用图标绘制方法
                        if let Err(e) = draw_binary_image(
                            &icon_id,
                            icon.position[0],
                            icon.position[1],
                            color,
                            &mut self.display,
                        ) {
                            log::warn!("Failed to render icon '{}': {:?}", icon.id.as_str(), e);
                        }
                    }
                    Err(_) => {
                        log::warn!(
                            "Invalid icon ID '{}' for element '{}'",
                            icon.icon_id.as_str(),
                            icon.id.as_str()
                        );
                    }
                }
            }
            LayoutNode::Line(line) => {
                // 确定线条颜色
                let color = self.importance_to_color(&line.importance);

                // 创建线条并绘制
                let line_primitive = Line::new(
                    Point::new(line.start[0] as i32, line.start[1] as i32),
                    Point::new(line.end[0] as i32, line.end[1] as i32),
                )
                .stroke_color(color.to_quad_color())
                .stroke_width(line.thickness as u32);

                if let Err(e) = line_primitive.draw(&mut self.display) {
                    log::warn!("Failed to render line '{}': {:?}", line.id.as_str(), e);
                }
            }
            LayoutNode::Rectangle(rect) => {
                // 创建矩形
                let rect_primitive = Rectangle::new(
                    Point::new(rect.rect[0] as i32, rect.rect[1] as i32),
                    Point::new(
                        (rect.rect[0] + rect.rect[2]) as i32 - 1,
                        (rect.rect[1] + rect.rect[3]) as i32 - 1,
                    ),
                );

                // 处理填充
                if let Some(fill_importance) = &rect.fill_importance {
                    let fill_color = self.importance_to_color(&Some(fill_importance.clone()));
                    let filled_rect = rect_primitive.fill_color(fill_color.to_quad_color());

                    if let Err(e) = filled_rect.draw(&mut self.display) {
                        log::warn!(
                            "Failed to render filled rectangle '{}': {:?}",
                            rect.id.as_str(),
                            e
                        );
                    }
                }

                // 处理描边
                if let Some(stroke_importance) = &rect.stroke_importance {
                    let stroke_color = self.importance_to_color(&Some(stroke_importance.clone()));
                    let stroked_rect = rect_primitive
                        .stroke_color(stroke_color.to_quad_color())
                        .stroke_width(rect.stroke_thickness as u32);

                    if let Err(e) = stroked_rect.draw(&mut self.display) {
                        log::warn!(
                            "Failed to render stroked rectangle '{}': {:?}",
                            rect.id.as_str(),
                            e
                        );
                    }
                }
            }
            LayoutNode::Circle(circle) => {
                // 创建圆形
                let circle_primitive = Circle::new(
                    Point::new(circle.center[0] as i32, circle.center[1] as i32),
                    circle.radius as u32,
                );

                // 处理填充
                if let Some(fill_importance) = &circle.fill_importance {
                    let fill_color = self.importance_to_color(&Some(fill_importance.clone()));
                    let filled_circle = circle_primitive.fill_color(fill_color.to_quad_color());

                    if let Err(e) = filled_circle.draw(&mut self.display) {
                        log::warn!(
                            "Failed to render filled circle '{}': {:?}",
                            circle.id.as_str(),
                            e
                        );
                    }
                }

                // 处理描边
                if let Some(stroke_importance) = &circle.stroke_importance {
                    let stroke_color = self.importance_to_color(&Some(stroke_importance.clone()));
                    let stroked_circle = circle_primitive
                        .stroke_color(stroke_color.to_quad_color())
                        .stroke_width(circle.stroke_thickness as u32);

                    if let Err(e) = stroked_circle.draw(&mut self.display) {
                        log::warn!(
                            "Failed to render stroked circle '{}': {:?}",
                            circle.id.as_str(),
                            e
                        );
                    }
                }
            }
        }
    }

    /// 将importance转换为颜色
    fn importance_to_color(&self, importance: &Option<String<16>>) -> EinkColor {
        match importance {
            Some(importance) => match importance.as_str() {
                "warning" => EinkColor::Yellow,
                "critical" => EinkColor::Red,
                _ => EinkColor::Black,
            },
            None => EinkColor::Black,
        }
    }

    /// 新的全屏渲染方法，基于布局文件
    pub fn render_full_screen(&mut self, state: &SystemState) -> Result<()> {
        log::info!("Starting full screen rendering using layout file");

        // 加载布局
        let layout = self.load_layout()?;

        // 递归渲染所有布局节点
        self.render_layout_node(&layout, state);

        log::info!("Full screen buffer rendering completed");
        Ok(())
    }

    /// 在屏幕上刷新显示，将内存中的内容显示出来
    pub async fn refresh_display(&mut self) -> Result<()> {
        log::info!("Refreshing display");

        self.init_driver().map_err(|_| {
            log::error!("Failed to initialize display driver");
            AppError::RenderingFailed
        })?;

        self.driver
            .update_frame(self.display.buffer())
            .map_err(|e| {
                log::error!("Failed to update frame: {}", e);
                AppError::RenderingFailed
            })?;

        self.driver.display_frame().map_err(|e| {
            log::error!("Failed to refresh display: {}", e);
            AppError::RenderingFailed
        })?;

        log::info!("Display refreshed successfully");
        Ok(())
    }
}
