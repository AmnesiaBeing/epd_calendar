//! 布局处理器模块（编译期执行）

use anyhow::{Context, Result, anyhow};
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::builder::config::BuildConfig;
use crate::builder::utils::progress::ProgressTracker;

// ========== 编译期核心数据结构 ==========
/// 编译期布局常数
#[derive(Debug, Clone)]
pub struct CompileLayoutConst {
    // 元素样式（key: 元素class蛇形名，value: 样式属性）
    pub element_styles: HashMap<String, ElementStyle>,
    // Flex容器配置（key: 元素class蛇形名）
    pub flex_containers: HashMap<String, FlexConfig>,
    // 元素层级关系（父class -> 子class列表）
    pub element_hierarchy: HashMap<String, Vec<String>>,
    // 动态值映射（HTML模板变量 -> 数据源路径）
    pub dynamic_mapping: HashMap<String, String>,
    // 元素类型映射（class -> 元素类型，编译期解析HTML时生成）
    pub element_type_mapping: HashMap<String, ElementType>,
}

/// 元素类型（编译期解析HTML标签/样式确定）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Icon,      // 图标元素（标签为img）
    Text,      // 文本元素（包含文本）
    Container, // 容器元素（Flex容器，无动态内容）
}

/// 元素样式（仅保留非默认值）
#[derive(Debug, Clone)]
pub struct ElementStyle {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub font_size: Option<u32>, // 只允许使用BuildConfig里面的字体大小
    pub margin: (Option<u32>, Option<u32>, Option<u32>, Option<u32>), // top, right, bottom, left
    pub padding: (Option<u32>, Option<u32>, Option<u32>, Option<u32>), // top, right, bottom, left
    pub text_align: Option<TextAlign>,
}

/// Flex布局配置，只解析以下Flex属性，其他属性不做解析
#[derive(Debug, Clone)]
pub struct FlexConfig {
    pub direction: FlexDir,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
}

/// 基础枚举（编译期仅用于数据存储）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDir {
    Column,
    Row,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Left,
    Center,
    SpaceAround,
    SpaceBetween,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Center,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// 编译期绘制指令（纯数据，无函数指针）
#[derive(Debug, Clone)]
pub struct CompileDrawInstruction {
    pub id: String,
    pub draw_type: DrawType,
    pub dynamic_key: String,     // 数据源路径
    pub calc_config: CalcConfig, // 计算规则配置
    pub font_size: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum DrawType {
    Icon,
    Text,
    Rectangle, // Line也是矩形的一种
}

#[derive(Debug, Clone)]
pub struct CalcConfig {
    pub x_calc_type: CalcType,
    pub y_calc_type: CalcType,
    pub width_calc_type: CalcType,
    pub height_calc_type: CalcType,
    pub parent_ref: String, // 参考父元素
}

/// 所有的编译期计算类型，根据不同类型生成编译期计算的代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcType {
    FlexCenterX,
    FlexRowX,
    FlexSpaceAroundX,
    FlexSpaceBetweenX,
    FlexColumnY,
    FlexRowY,
    TextVerticalCenterY,
    FixedWidth,
    FixedHeight,
    TextWidth,
    MultiLineTextHeight,
    ImageWidth,
    ImageHeight,
}

// ========== 编译期HTML解析逻辑 ==========
/// 解析HTML和样式
pub fn parse_html_layout(html_path: &Path) -> Result<CompileLayoutConst> {
    // 1. 读取HTML文件
    let html_content = fs::read_to_string(html_path)
        .with_context(|| format!("读取HTML文件失败: {}", html_path.display()))?;
    let document = Html::parse_document(&html_content);

    // 2. 解析<style>标签中的样式
    let (class_styles, flex_configs) = parse_style_tags(&document)?;

    // 3. 解析根容器
    let root_const = parse_root_const(&class_styles)?;

    // 4. 解析元素层级关系
    let element_hierarchy = parse_element_hierarchy(&document)?;

    // 5. 解析动态值映射（{{变量}} -> 数据源路径）
    let dynamic_mapping = parse_dynamic_mapping(&html_content)?;

    // 6. 解析元素类型（核心：为动态生成指令做准备）
    let element_type_mapping = parse_element_types(&document, &class_styles, &dynamic_mapping)?;

    Ok(CompileLayoutConst {
        root: root_const,
        element_styles: class_styles,
        flex_containers: flex_configs,
        element_hierarchy,
        dynamic_mapping,
        element_type_mapping,
    })
}

/// 解析元素类型（核心：区分图标/文本/形状/容器）
fn parse_element_types(
    document: &Html,
    class_styles: &HashMap<String, ElementStyle>,
    dynamic_mapping: &HashMap<String, String>,
) -> Result<HashMap<String, ElementType>> {
    let mut element_type_mapping = HashMap::new();
    let all_elements_selector = Selector::parse("[class]").unwrap(); // 匹配所有带class的元素

    for elem in document.select(&all_elements_selector) {
        // 获取元素class（转蛇形）
        let classes: Vec<String> = elem
            .value()
            .classes()
            .map(|c| c.replace('-', "_"))
            .filter(|c| !c.is_empty())
            .collect();

        if classes.is_empty() {
            continue;
        }

        let main_class = &classes[0];

        // 判断图标元素（标签为img）
        let tag_name = elem.value().name();
        let is_icon = tag_name == "img";

        if is_icon {
            element_type_mapping.insert(main_class.clone(), ElementType::Icon);
            continue;
        }

        // 判断文本元素（有font_size样式，或关联动态文本变量）
        let has_font_style = class_styles
            .get(main_class)
            .map(|s| s.font_size.is_some())
            .unwrap_or(false);

        // 检查元素是否包含动态文本变量（通过元素文本内容匹配）
        let elem_text = elem.text().collect::<String>();
        let has_dynamic_text = dynamic_mapping
            .keys()
            .any(|var| elem_text.contains(&format!("{{{{{}}}}}", var)));

        if has_font_style || has_dynamic_text {
            element_type_mapping.insert(main_class.clone(), ElementType::Text);
            continue;
        }

        // 检查元素是否有背景颜色
        let has_background_color = class_styles
            .get(main_class)
            .map(|s| s.background_color.is_some())
            .unwrap_or(false);

        if has_background_color {
            element_type_mapping.insert(main_class.clone(), ElementType::Shape);
            continue;
        }

        // 剩余为容器元素（如Flex容器）
        element_type_mapping.insert(main_class.clone(), ElementType::Container);
    }

    Ok(element_type_mapping)
}

/// 解析<style>标签中的所有class样式
fn parse_style_tags(
    document: &Html,
) -> Result<(HashMap<String, ElementStyle>, HashMap<String, FlexConfig>)> {
    let mut class_styles = HashMap::new();
    let mut flex_configs = HashMap::new();

    // 选择所有style标签
    let style_selector = Selector::parse("style").unwrap();
    for style_elem in document.select(&style_selector) {
        let style_content = style_elem.text().collect::<String>();
        // 分割CSS规则（简化解析，实际项目建议用cssparser库）
        let css_rules = split_css_rules(&style_content);

        for rule in css_rules {
            if rule.trim().is_empty() {
                continue;
            }

            // 解析选择器和样式块
            let (selector_part, style_part) = rule
                .split_once('{')
                .ok_or_else(|| anyhow!("无效的CSS规则: {}", rule))?;
            let style_part = style_part.trim_end_matches('}').trim();

            // 仅处理class选择器（.xxx）
            let selectors = selector_part.split(',').map(|s| s.trim());
            for selector in selectors {
                if selector.starts_with('.') {
                    let class_name = selector.trim_start_matches('.').replace('-', "_");
                    let styles = parse_css_declarations(style_part)?;

                    // 解析普通样式
                    let element_style = parse_element_style(&styles)?;
                    class_styles.insert(class_name.clone(), element_style);

                    // 解析Flex配置
                    if let Some(flex_config) = parse_flex_config(&styles)? {
                        flex_configs.insert(class_name.clone(), flex_config);
                    }
                }
            }
        }
    }

    Ok((class_styles, flex_configs))
}

/// 分割CSS规则（修复生命周期错误，返回String而非&str）
fn split_css_rules(css: &str) -> Vec<String> {
    let mut rules = Vec::new();
    let mut current_rule = String::new();
    let mut brace_depth = 0;

    for c in css.chars() {
        match c {
            '{' => {
                brace_depth += 1;
                current_rule.push(c);
            }
            '}' => {
                brace_depth -= 1;
                current_rule.push(c);
                if brace_depth == 0 {
                    // 转成String，避免悬垂引用
                    let trimmed_rule = current_rule.trim().to_string();
                    if !trimmed_rule.is_empty() {
                        rules.push(trimmed_rule);
                    }
                    current_rule.clear();
                }
            }
            _ if brace_depth > 0 => {
                current_rule.push(c);
            }
            _ => {}
        }
    }

    rules
}

/// 解析CSS声明（key:value;）
fn parse_css_declarations(style_part: &str) -> Result<HashMap<String, String>> {
    let mut styles = HashMap::new();

    for decl in style_part.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }

        let (key, value) = decl
            .split_once(':')
            .ok_or_else(|| anyhow!("无效的CSS声明: {}", decl))?;
        let key = key.trim().to_lowercase();
        let value = value.trim().to_string();
        styles.insert(key, value);
    }

    Ok(styles)
}

/// 解析元素样式（过滤默认值）
fn parse_element_style(styles: &HashMap<String, String>) -> Result<ElementStyle> {
    Ok(ElementStyle {
        width: parse_px_value(styles.get("width")),
        height: parse_px_value(styles.get("height")),
        font_size: parse_px_value(styles.get("font-size")),
        margin: (
            parse_px_value(styles.get("margin-top")),
            parse_px_value(styles.get("margin-right")),
            parse_px_value(styles.get("margin-bottom")),
            parse_px_value(styles.get("margin-left")),
        ),
        padding: (
            parse_px_value(styles.get("padding-top")),
            parse_px_value(styles.get("padding-right")),
            parse_px_value(styles.get("padding-bottom")),
            parse_px_value(styles.get("padding-left")),
        ),
        text_align: parse_text_align(styles.get("text-align")),
    })
}

/// 解析Flex配置
fn parse_flex_config(styles: &HashMap<String, String>) -> Result<Option<FlexConfig>> {
    // 仅处理display:flex的元素
    let display = styles.get("display").map(|s| s.to_lowercase());
    if display != Some("flex".to_string()) {
        return Ok(None);
    }

    Ok(Some(FlexConfig {
        direction: parse_flex_dir(styles.get("flex-direction")),
        justify_content: parse_justify_content(styles.get("justify-content")),
        align_items: parse_align_items(styles.get("align-items")),
    }))
}

/// 根容器常数不需要解析，这是一个常数
fn parse_root_const(_class_styles: &HashMap<String, ElementStyle>) -> Result<RootConst> {
    Ok(RootConst {
        width: 800,
        height: 480,
    })
}

/// 解析元素层级关系
fn parse_element_hierarchy(document: &Html) -> Result<HashMap<String, Vec<String>>> {
    let mut hierarchy = HashMap::new();
    let root_selector = Selector::parse(".root_container").unwrap();

    // 递归遍历根容器的所有子元素
    if let Some(root_elem) = document.select(&root_selector).next() {
        traverse_element_hierarchy(
            ElementRef::wrap(*root_elem).unwrap(),
            "root_container",
            &mut hierarchy,
        )?;
    }

    Ok(hierarchy)
}

/// 递归遍历元素层级
fn traverse_element_hierarchy(
    elem: ElementRef,
    parent_class: &str,
    hierarchy: &mut HashMap<String, Vec<String>>,
) -> Result<()> {
    // 获取当前元素的class（转蛇形）
    let current_classes: Vec<String> = elem
        .value()
        .classes()
        .map(|c| c.replace('-', "_"))
        .filter(|c| !c.is_empty())
        .collect();

    if current_classes.is_empty() {
        return Ok(());
    }

    // 取第一个非空class作为标识
    let current_class = &current_classes[0];

    // 添加到父元素的子列表
    hierarchy
        .entry(parent_class.to_string())
        .or_default()
        .push(current_class.clone());

    // 递归处理子元素
    for child in elem.children() {
        if let Some(child_elem) = ElementRef::wrap(child) {
            traverse_element_hierarchy(child_elem, current_class, hierarchy)?;
        }
    }

    Ok(())
}

/// 解析动态值映射（{{变量}} -> 数据源路径）
/// 直接处理原始HTML字符串，无需scraper::Html结构体
fn parse_dynamic_mapping(html_content: &str) -> Result<HashMap<String, String>> {
    let mut mapping = HashMap::new();
    let mut chars = html_content.chars().peekable();

    while let Some(&c) = chars.peek() {
        // 匹配 {{ 开头
        if c == '{' {
            chars.next();
            if chars.peek() == Some(&'{') {
                chars.next();
                // 解析变量名（直到 }} 结束）
                let mut var_name = String::new();
                let mut is_closed = false;

                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if chars.peek() == Some(&'}') {
                            chars.next();
                            is_closed = true;
                            break;
                        }
                        // 单个 }，不是结束符，加回去
                        var_name.push('}');
                    } else {
                        var_name.push(c);
                        chars.next();
                    }
                }

                // 仅处理完整闭合的变量
                if is_closed && !var_name.is_empty() {
                    let trimmed_var = var_name.trim().to_string();
                    // 变量名 -> 数据源路径（保持一致，可根据需求自定义映射规则）
                    mapping.insert(trimmed_var.clone(), trimmed_var);
                }
            }
        } else {
            chars.next();
        }
    }

    Ok(mapping)
}

// ========== 编译期辅助解析函数 ==========
/// 解析px数值（过滤默认值0）
fn parse_px_value(value: Option<&String>) -> Option<u32> {
    value.and_then(|v| {
        let v = v.trim().replace("px", "").replace("%", "");
        // 处理百分比（基于根容器800x480）
        if v.ends_with('%') {
            let num = v.trim_end_matches('%').parse::<f32>().ok()?;
            let px = (num / 100.0 * 800.0) as u32;
            if px > 0 { Some(px) } else { None }
        } else {
            let num = v.parse::<u32>().ok()?;
            if num > 0 { Some(num) } else { None }
        }
    })
}

/// 解析浮点数
fn parse_float_value(value: Option<&String>) -> Option<f32> {
    value.and_then(|v| v.parse::<f32>().ok())
}

/// 解析颜色值
fn parse_color_value(value: Option<&String>) -> Option<u32> {
    value.and_then(|v| {
        let v = v.trim().strip_prefix('#').unwrap_or(v);
        if v.len() == 6 {
            u32::from_str_radix(v, 16).ok()
        } else {
            None
        }
    })
}

/// 解析Flex方向
fn parse_flex_dir(value: Option<&String>) -> FlexDir {
    value.map_or(FlexDir::Row, |v| match v.trim().to_lowercase().as_str() {
        "column" | "column-reverse" => FlexDir::Column,
        _ => FlexDir::Row,
    })
}

/// 解析JustifyContent
fn parse_justify_content(value: Option<&String>) -> JustifyContent {
    value.map_or(JustifyContent::Left, |v| {
        match v.trim().to_lowercase().as_str() {
            "center" => JustifyContent::Center,
            "space-around" => JustifyContent::SpaceAround,
            "space-between" => JustifyContent::SpaceBetween,
            _ => JustifyContent::Left,
        }
    })
}

/// 解析AlignItems
fn parse_align_items(value: Option<&String>) -> AlignItems {
    value.map_or(AlignItems::Center, |v| {
        match v.trim().to_lowercase().as_str() {
            "stretch" => AlignItems::Stretch,
            _ => AlignItems::Center,
        }
    })
}

/// 解析文本对齐
fn parse_text_align(value: Option<&String>) -> Option<TextAlign> {
    value.map(|v| match v.trim().to_lowercase().as_str() {
        "center" => TextAlign::Center,
        "right" => TextAlign::Right,
        _ => TextAlign::Left,
    })
}

// ========== 编译期生成运行时代码 ==========
/// 生成运行时布局代码（包含函数指针）
pub fn generate_runtime_code(compile_const: &CompileLayoutConst, output_dir: &Path) -> Result<()> {
    // 1. 生成运行时布局常数
    let const_code = generate_layout_const_code(compile_const)?;

    // 2. 生成计算函数代码
    let calc_code = generate_calc_functions_code()?;
    let instructions_code = generate_draw_instructions_code(compile_const)?;

    // 3. 合并代码并写入文件
    let mut code = String::new();
    code.push_str("//! 自动生成的运行时布局代码\n");
    code.push_str("//! 编译期生成，运行时调用\n");
    code.push_str("#![allow(dead_code)]\n");
    code.push_str("#![allow(clippy::unreadable_literal)]\n\n");

    // 导入运行时依赖
    code.push_str("use std::collections::{HashMap, BTreeMap};\n");
    code.push_str("use crate::assets::generated_fonts::FontSize;\n");
    code.push_str("use crate::kernel::data::scheduler::DataSourceRegistry;\n");
    code.push_str("use crate::kernel::data::types::CacheKeyValueMap;\n");
    code.push_str("use crate::common::error::{AppError, Result};\n");
    code.push_str("use crate::kernel::render::text_renderer::TextRenderer;\n");
    code.push_str("use crate::kernel::render::icon_renderer::IconRenderer;\n");
    code.push_str("use crate::assets::generated_icons::IconId;\n\n");

    // 添加类型定义
    code.push_str(&generate_type_definitions()?);
    // 添加布局常数
    code.push_str(&const_code);
    // 添加计算函数
    code.push_str(&calc_code);
    // 添加绘制指令集
    code.push_str(&instructions_code);

    // 写入文件
    let output_path = output_dir.join("generated_layouts.rs");
    fs::write(&output_path, code)
        .with_context(|| format!("写入布局代码失败: {}", output_path.display()))?;

    Ok(())
}

/// 生成类型定义代码
fn generate_type_definitions() -> Result<String> {
    let mut code = String::new();

    // 运行时枚举定义
    code.push_str("// ========== 核心枚举定义 ==========\n");
    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    code.push_str("pub enum FlexDir {\n");
    code.push_str("    Column,\n");
    code.push_str("    Row,\n");
    code.push_str("}\n\n");

    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    code.push_str("pub enum JustifyContent {\n");
    code.push_str("    Left,\n");
    code.push_str("    Center,\n");
    code.push_str("    SpaceAround,\n");
    code.push_str("    SpaceBetween,\n");
    code.push_str("}\n\n");

    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    code.push_str("pub enum AlignItems {\n");
    code.push_str("    Center,\n");
    code.push_str("    Stretch,\n");
    code.push_str("}\n\n");

    // 函数指针类型定义
    code.push_str("// ========== 函数指针类型定义 ==========\n");
    code.push_str("/// 动态值获取函数类型（对接DataSourceRegistry）\n");
    code.push_str(
        "pub type GetDynamicValueFn = fn(&CacheKeyValueMap, &str) -> Result<String>;\n\n",
    );

    code.push_str("/// 坐标/尺寸计算函数指针类型\n");
    code.push_str(
        "pub type XCalcFn = fn(&LayoutConst, &GetDynamicValueFn, &CacheKeyValueMap) -> u32;\n",
    );
    code.push_str(
        "pub type YCalcFn = fn(&LayoutConst, &GetDynamicValueFn, &CacheKeyValueMap) -> u32;\n",
    );
    code.push_str(
        "pub type WidthCalcFn = fn(&LayoutConst, &GetDynamicValueFn, &CacheKeyValueMap) -> u32;\n",
    );
    code.push_str("pub type HeightCalcFn = fn(&LayoutConst, &GetDynamicValueFn, &CacheKeyValueMap) -> u32;\n\n");

    // 绘制指令类型
    code.push_str("// ========== 绘制指令类型 ==========\n");
    code.push_str("#[derive(Debug, Clone)]\n");
    code.push_str("pub enum DrawType {\n");
    code.push_str("    SvgIcon,\n");
    code.push_str("    Text,\n");
    code.push_str("    Divider,\n");
    code.push_str("}\n\n");

    code.push_str("#[derive(Debug, Clone)]\n");
    code.push_str("pub struct DrawInstruction {\n");
    code.push_str("    pub id: String,\n");
    code.push_str("    pub draw_type: DrawType,\n");
    code.push_str("    pub dynamic_key: String,\n");
    code.push_str("    pub x_calc: XCalcFn,\n");
    code.push_str("    pub y_calc: YCalcFn,\n");
    code.push_str("    pub width_calc: Option<WidthCalcFn>,\n");
    code.push_str("    pub height_calc: Option<HeightCalcFn>,\n");
    code.push_str("    pub font_size: Option<FontSize>,\n");
    code.push_str("    pub line_height: Option<f32>,\n");
    code.push_str("}\n\n");

    Ok(code)
}

/// 生成布局常数代码
fn generate_layout_const_code(compile_const: &CompileLayoutConst) -> Result<String> {
    let mut code = String::new();

    // 根容器常数
    code.push_str("// ========== 布局常数 ==========\n");
    code.push_str("#[derive(Debug, Clone)]\n");
    code.push_str("pub struct LayoutConst {\n");
    code.push_str("    // 根容器常数\n");
    code.push_str("    pub root_width: u32,\n");
    code.push_str("    pub root_height: u32,\n");
    code.push_str("    pub root_padding_left: u32,\n");
    code.push_str("    pub root_padding_top: u32,\n");
    code.push_str("    pub root_padding_right: u32,\n");
    code.push_str("    pub root_padding_bottom: u32,\n");
    code.push_str("    // 元素样式缓存\n");
    code.push_str("    pub element_styles: HashMap<String, ElementStyle>,\n");
    code.push_str("    // Flex容器配置\n");
    code.push_str("    pub flex_containers: HashMap<String, FlexConfig>,\n");
    code.push_str("    // 分割线配置\n");
    code.push_str("    pub dividers: HashMap<String, DividerConfig>,\n");
    code.push_str("}\n\n");

    // 元素样式结构体
    code.push_str("#[derive(Debug, Clone)]\n");
    code.push_str("pub struct ElementStyle {\n");
    code.push_str("    pub width: Option<u32>,\n");
    code.push_str("    pub height: Option<u32>,\n");
    code.push_str("    pub font_size: Option<u32>,\n");
    code.push_str("    pub margin_top: Option<u32>,\n");
    code.push_str("    pub margin_right: Option<u32>,\n");
    code.push_str("    pub margin_bottom: Option<u32>,\n");
    code.push_str("    pub margin_left: Option<u32>,\n");
    code.push_str("    pub padding_top: Option<u32>,\n");
    code.push_str("    pub padding_right: Option<u32>,\n");
    code.push_str("    pub padding_bottom: Option<u32>,\n");
    code.push_str("    pub padding_left: Option<u32>,\n");
    code.push_str("    pub line_height: Option<f32>,\n");
    code.push_str("    pub text_align: Option<TextAlign>,\n");
    code.push_str("}\n\n");

    // Flex配置结构体
    code.push_str("#[derive(Debug, Clone)]\n");
    code.push_str("pub struct FlexConfig {\n");
    code.push_str("    pub direction: FlexDir,\n");
    code.push_str("    pub justify_content: JustifyContent,\n");
    code.push_str("    pub align_items: AlignItems,\n");
    code.push_str("    pub gap: Option<u32>,\n");
    code.push_str("}\n\n");

    // 分割线配置结构体
    code.push_str("#[derive(Debug, Clone)]\n");
    code.push_str("pub struct DividerConfig {\n");
    code.push_str("    pub thickness: u32,\n");
    code.push_str("    pub color: u32,\n");
    code.push_str("    pub is_vertical: bool,\n");
    code.push_str("}\n\n");

    // 文本对齐枚举
    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    code.push_str("pub enum TextAlign {\n");
    code.push_str("    Left,\n");
    code.push_str("    Center,\n");
    code.push_str("    Right,\n");
    code.push_str("}\n\n");

    // 全局布局常数实例
    code.push_str("/// 全局布局常数实例\n");
    code.push_str("pub static LAYOUT_CONST: LayoutConst = LayoutConst {\n");
    code.push_str(&format!("    root_width: {},\n", compile_const.root.width));
    code.push_str(&format!(
        "    root_height: {},\n",
        compile_const.root.height
    ));
    code.push_str("    element_styles: {\n");
    code.push_str("        let mut map = HashMap::new();\n");

    // 生成元素样式
    for (class, style) in &compile_const.element_styles {
        code.push_str(&format!(
            "        map.insert(\"{}\".to_string(), ElementStyle {{\n",
            class
        ));
        code.push_str(&format!("            width: {:?},\n", style.width));
        code.push_str(&format!("            height: {:?},\n", style.height));
        code.push_str(&format!("            font_size: {:?},\n", style.font_size));
        code.push_str(&format!("            margin_top: {:?},\n", style.margin.0));
        code.push_str(&format!(
            "            margin_right: {:?},\n",
            style.margin.1
        ));
        code.push_str(&format!(
            "            margin_bottom: {:?},\n",
            style.margin.2
        ));
        code.push_str(&format!("            margin_left: {:?},\n", style.margin.3));
        code.push_str(&format!(
            "            padding_top: {:?},\n",
            style.padding.0
        ));
        code.push_str(&format!(
            "            padding_right: {:?},\n",
            style.padding.1
        ));
        code.push_str(&format!(
            "            padding_bottom: {:?},\n",
            style.padding.2
        ));
        code.push_str(&format!(
            "            padding_left: {:?},\n",
            style.padding.3
        ));
        code.push_str(&format!(
            "            text_align: {:?},\n",
            style.text_align
        ));
        code.push_str("        });\n");
    }

    code.push_str("        map\n");
    code.push_str("    },\n");

    // 生成Flex容器配置
    code.push_str("    flex_containers: {\n");
    code.push_str("        let mut map = HashMap::new();\n");
    for (class, config) in &compile_const.flex_containers {
        code.push_str(&format!(
            "        map.insert(\"{}\".to_string(), FlexConfig {{\n",
            class
        ));
        code.push_str(&format!(
            "            direction: FlexDir::{:?},\n",
            config.direction
        ));
        code.push_str(&format!(
            "            justify_content: JustifyContent::{:?},\n",
            config.justify_content
        ));
        code.push_str(&format!(
            "            align_items: AlignItems::{:?},\n",
            config.align_items
        ));
        code.push_str("        });\n");
    }
    code.push_str("        map\n");
    code.push_str("    },\n");

    code.push_str("};\n\n");

    Ok(code)
}

/// 生成计算函数代码
fn generate_calc_functions_code() -> Result<String> {
    let mut code = String::new();

    code.push_str("// ========== 布局计算函数 ==========\n");

    // Flex Center X 计算
    code.push_str("/// Flex水平居中X坐标计算\n");
    code.push_str("pub fn flex_center_x(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    // 获取父容器宽度\n");
    code.push_str("    let parent_width = consts.root_width - consts.root_padding_left - consts.root_padding_right;\n");
    code.push_str("    // 简化实现：居中偏移\n");
    code.push_str("    consts.root_padding_left + (parent_width / 2)\n");
    code.push_str("}\n\n");

    // Flex Row X 计算
    code.push_str("/// Flex Row布局X坐标计算（基础）\n");
    code.push_str("pub fn flex_row_x(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    consts.root_padding_left\n");
    code.push_str("}\n\n");

    // Flex Space Around X 计算
    code.push_str("/// Flex SpaceAround X坐标计算\n");
    code.push_str("pub fn flex_space_around_x(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let parent_width = consts.root_width - consts.root_padding_left - consts.root_padding_right;\n");
    code.push_str("    // 简化实现：均分间距\n");
    code.push_str("    consts.root_padding_left + (parent_width / 4)\n");
    code.push_str("}\n\n");

    // Flex Space Between X 计算
    code.push_str("/// Flex SpaceBetween X坐标计算\n");
    code.push_str("pub fn flex_space_between_x(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let parent_width = consts.root_width - consts.root_padding_left - consts.root_padding_right;\n");
    code.push_str("    consts.root_padding_left + (parent_width / 3)\n");
    code.push_str("}\n\n");

    // Flex Row Y 计算
    code.push_str("/// Flex Row布局Y坐标计算\n");
    code.push_str("pub fn flex_row_y(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    consts.root_padding_top\n");
    code.push_str("}\n\n");

    // Flex Column Y 计算
    code.push_str("/// Flex Column布局Y坐标计算\n");
    code.push_str("pub fn flex_column_y(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    consts.root_padding_top + 20\n");
    code.push_str("}\n\n");

    // 文本垂直居中Y计算
    code.push_str("/// 文本垂直居中Y坐标计算\n");
    code.push_str("pub fn text_vertical_center_y(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    // 动态获取元素高度（从样式中读取）\n");
    code.push_str("    pub fn get_element_height(consts: &LayoutConst, elem_id: &str) -> u32 {\n");
    code.push_str("        consts.element_styles.get(elem_id)\n");
    code.push_str("            .and_then(|s| s.height)\n");
    code.push_str("            .unwrap_or(30)\n");
    code.push_str("    }\n");
    code.push_str("    // 动态获取字体大小\n");
    code.push_str(
        "    pub fn get_element_font_size(consts: &LayoutConst, elem_id: &str) -> u32 {\n",
    );
    code.push_str("        consts.element_styles.get(elem_id)\n");
    code.push_str("            .and_then(|s| s.font_size)\n");
    code.push_str("            .unwrap_or(24)\n");
    code.push_str("    }\n");
    code.push_str("    // 从调用上下文获取当前元素ID（运行时可通过指令ID解析）\n");
    code.push_str(
        "    let elem_id = cache.get(\"current_elem_id\").unwrap_or(\"text_element\");\n",
    );
    code.push_str("    let parent_height = get_element_height(consts, elem_id);\n");
    code.push_str("    let font_size = get_element_font_size(consts, elem_id);\n");
    code.push_str("    // 垂直居中计算\n");
    code.push_str("    consts.root_padding_top + (parent_height - font_size) / 2\n");
    code.push_str("}\n\n");

    // 固定宽度计算
    code.push_str("/// 固定宽度计算\n");
    code.push_str("pub fn fixed_width(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let elem_id = cache.get(\"current_elem_id\").unwrap_or(\"default\");\n");
    code.push_str("    consts.element_styles.get(elem_id)\n");
    code.push_str("        .and_then(|s| s.width)\n");
    code.push_str("        .unwrap_or(0)\n");
    code.push_str("}\n\n");

    // 固定高度计算
    code.push_str("/// 固定高度计算\n");
    code.push_str("pub fn fixed_height(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let elem_id = cache.get(\"current_elem_id\").unwrap_or(\"default\");\n");
    code.push_str("    consts.element_styles.get(elem_id)\n");
    code.push_str("        .and_then(|s| s.height)\n");
    code.push_str("        .unwrap_or(0)\n");
    code.push_str("}\n\n");

    // 文本宽度计算
    code.push_str("/// 文本宽度计算（对接TextRenderer）\n");
    code.push_str("pub fn text_width(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    // 获取动态文本内容\n");
    code.push_str("    let dynamic_key = cache.get(\"current_dynamic_key\").unwrap_or(\"\");\n");
    code.push_str("    let text = get_dynamic(cache, dynamic_key).unwrap_or_default();\n");
    code.push_str("    // 动态获取字体大小\n");
    code.push_str(
        "    let elem_id = cache.get(\"current_elem_id\").unwrap_or(\"text_element\");\n",
    );
    code.push_str("    let font_size_px = consts.element_styles.get(elem_id)\n");
    code.push_str("        .and_then(|s| s.font_size)\n");
    code.push_str("        .unwrap_or(24);\n");
    code.push_str("    // 转换为FontSize枚举（适配项目定义）\n");
    code.push_str("    let font_size = match font_size_px {\n");
    code.push_str("        12 => FontSize::Small,\n");
    code.push_str("        24 => FontSize::Medium,\n");
    code.push_str("        32 => FontSize::Large,\n");
    code.push_str("        _ => FontSize::Medium,\n");
    code.push_str("    };\n");
    code.push_str("    // 调用TextRenderer计算宽度\n");
    code.push_str("    TextRenderer::calculate_text_width(&text, font_size).unwrap_or(0)\n");
    code.push_str("}\n\n");

    // 多行文本高度计算
    code.push_str("/// 多行文本高度计算\n");
    code.push_str("pub fn multi_line_text_height(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let text_width = text_width(consts, get_dynamic, cache);\n");
    code.push_str("    let line_height = consts.element_styles.get(cache.get(\"current_elem_id\").unwrap_or(\"text_element\"))\n");
    code.push_str("        .and_then(|s| s.line_height)\n");
    code.push_str("        .unwrap_or(1.2);\n");
    code.push_str("    // 简化计算：按宽度折行\n");
    code.push_str("    let line_count = (text_width / 200) + 1;\n");
    code.push_str("    (line_count as f32 * line_height * 24.0) as u32\n");
    code.push_str("}\n\n");

    // 图片尺寸计算
    code.push_str("/// 图片宽度计算（对接IconId）\n");
    code.push_str("pub fn image_width(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let dynamic_key = cache.get(\"current_dynamic_key\").unwrap_or(\"\");\n");
    code.push_str("    let icon_path = get_dynamic(cache, dynamic_key).unwrap_or_default();\n");
    code.push_str("    // 从IconId获取尺寸\n");
    code.push_str("    IconId::get_icon_data(&icon_path)\n");
    code.push_str("        .map(|icon| icon.size().width)\n");
    code.push_str("        .unwrap_or(48)\n");
    code.push_str("}\n\n");

    code.push_str("/// 图片高度计算\n");
    code.push_str("pub fn image_height(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let dynamic_key = cache.get(\"current_dynamic_key\").unwrap_or(\"\");\n");
    code.push_str("    let icon_path = get_dynamic(cache, dynamic_key).unwrap_or_default();\n");
    code.push_str("    IconId::get_icon_data(&icon_path)\n");
    code.push_str("        .map(|icon| icon.size().height)\n");
    code.push_str("        .unwrap_or(64)\n");
    code.push_str("}\n\n");

    // 分割线计算函数
    code.push_str("/// 分割线X坐标计算\n");
    code.push_str("pub fn divider_x(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    consts.root_padding_left\n");
    code.push_str("}\n\n");

    code.push_str("/// 分割线Y坐标计算\n");
    code.push_str("pub fn divider_y(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let elem_id = cache.get(\"current_elem_id\").unwrap_or(\"divider\");\n");
    code.push_str("    // 从父元素高度计算位置\n");
    code.push_str("    let parent_height = consts.element_styles.get(\"root_container\")\n");
    code.push_str("        .and_then(|s| s.height)\n");
    code.push_str("        .unwrap_or(480);\n");
    code.push_str("    // 动态分割线位置（示例：按元素层级）\n");
    code.push_str("    match elem_id {\n");
    code.push_str("        \"divider_1\" => consts.root_padding_top + 50,\n");
    code.push_str("        \"divider_2\" => consts.root_padding_top + 150,\n");
    code.push_str("        _ => consts.root_padding_top + 100,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("/// 分割线宽度计算\n");
    code.push_str("pub fn divider_width(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let elem_id = cache.get(\"current_elem_id\").unwrap_or(\"divider\");\n");
    code.push_str("    let divider_config = consts.dividers.get(elem_id);\n");
    code.push_str("    if let Some(divider) = divider_config {\n");
    code.push_str("        if divider.is_vertical {\n");
    code.push_str("            divider.thickness\n");
    code.push_str("        } else {\n");
    code.push_str(
        "            consts.root_width - consts.root_padding_left - consts.root_padding_right\n",
    );
    code.push_str("        }\n");
    code.push_str("    } else {\n");
    code.push_str(
        "        consts.root_width - consts.root_padding_left - consts.root_padding_right\n",
    );
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("/// 分割线高度计算\n");
    code.push_str("pub fn divider_height(\n");
    code.push_str("    consts: &LayoutConst,\n");
    code.push_str("    get_dynamic: &GetDynamicValueFn,\n");
    code.push_str("    cache: &CacheKeyValueMap\n");
    code.push_str(") -> u32 {\n");
    code.push_str("    let elem_id = cache.get(\"current_elem_id\").unwrap_or(\"divider\");\n");
    code.push_str("    consts.dividers.get(elem_id)\n");
    code.push_str("        .map(|d| d.thickness)\n");
    code.push_str("        .unwrap_or(1)\n");
    code.push_str("}\n\n");

    Ok(code)
}

/// 生成绘制指令集代码（核心修复：基于compile_const动态生成）
fn generate_draw_instructions_code(compile_const: &CompileLayoutConst) -> Result<String> {
    let mut code = String::new();

    code.push_str("// ========== 绘制指令集 ==========\n");
    code.push_str("/// 全局绘制指令集（编译期动态生成）\n");
    code.push_str("pub static DRAW_INSTRUCTIONS: &[DrawInstruction] = &[\n");

    // 步骤1：遍历所有元素，筛选出需要生成指令的类型（排除容器）
    let drawable_elements: Vec<(&String, &ElementType)> = compile_const
        .element_type_mapping
        .iter()
        .filter(|&(_, &elem_type)| elem_type == ElementType::Icon || elem_type == ElementType::Text)
        .collect();

    // 步骤2：为每个可绘制元素生成指令
    for (elem_class, &elem_type) in drawable_elements {
        // 生成唯一ID（父元素_当前元素）
        let root_container = "root_container".to_string();
        let parent_class = compile_const
            .element_hierarchy
            .iter()
            .find(|(_, children)| children.contains(elem_class))
            .map(|(parent, _)| parent)
            .unwrap_or(&root_container);
        let instruction_id = format!("{}_{}", parent_class, elem_class);

        // 确定动态key（从dynamic_mapping匹配元素关联的变量）
        let dynamic_key = compile_const
            .dynamic_mapping
            .keys()
            .find(|var| {
                // 匹配规则：变量名包含元素class，或元素class包含变量名
                var.contains(elem_class) || elem_class.contains(*var)
            })
            .unwrap_or(&"".to_string())
            .clone();

        // 确定元素样式
        let elem_style = compile_const.element_styles.get(elem_class);
        let font_size_px = elem_style.and_then(|s| s.font_size);

        // 确定Flex配置（父容器）
        let parent_flex_config = compile_const.flex_containers.get(parent_class);

        // 根据元素类型和Flex配置确定计算函数
        let (draw_type, x_calc_fn, y_calc_fn, width_calc_fn, height_calc_fn) = match elem_type {
            ElementType::Icon => (
                "DrawType::SvgIcon",
                get_flex_x_calc_fn(parent_flex_config),
                "flex_row_y",
                Some("image_width"),
                Some("image_height"),
            ),
            ElementType::Text => (
                "DrawType::Text",
                get_flex_x_calc_fn(parent_flex_config),
                "text_vertical_center_y",
                Some("text_width"),
                Some("fixed_height"),
            ),
            _ => unreachable!("已过滤容器元素"),
        };

        // 转换字体大小为FontSize枚举
        let font_size_code = match font_size_px {
            Some(12) => "Some(FontSize::Small)",
            Some(24) => "Some(FontSize::Medium)",
            Some(32) => "Some(FontSize::Large)",
            Some(_) => "Some(FontSize::Medium)",
            None => "None",
        };

        // 生成指令代码
        code.push_str(&format!("    // {} 元素\n", elem_class));
        code.push_str(&format!("    DrawInstruction {{\n"));
        code.push_str(&format!(
            "        id: \"{}\".to_string(),\n",
            instruction_id
        ));
        code.push_str(&format!("        draw_type: {},\n", draw_type));
        code.push_str(&format!(
            "        dynamic_key: \"{}\".to_string(),\n",
            dynamic_key
        ));
        code.push_str(&format!("        x_calc: {},\n", x_calc_fn));
        code.push_str(&format!("        y_calc: {},\n", y_calc_fn));
        code.push_str(&format!(
            "        width_calc: {},\n",
            width_calc_fn.map_or("None".to_string(), |fn_name| format!("Some({})", fn_name))
        ));
        code.push_str(&format!(
            "        height_calc: {},\n",
            height_calc_fn.map_or("None".to_string(), |fn_name| format!("Some({})", fn_name))
        ));
        code.push_str(&format!("        font_size: {},\n", font_size_code));
        code.push_str("    },\n");
    }

    code.push_str("];\n\n");

    // 动态值获取函数实现（对接DataSourceRegistry）
    code.push_str("// ========== 动态值获取 ==========\n");
    code.push_str("/// 默认动态值获取函数（对接DataSourceRegistry）\n");
    code.push_str("pub fn default_get_dynamic_value(\n");
    code.push_str("    cache: &CacheKeyValueMap,\n");
    code.push_str("    path: &str\n");
    code.push_str(") -> Result<String> {\n");
    code.push_str("    DataSourceRegistry::get_value_by_path_sync(cache, path)\n");
    code.push_str("        .map(|v| v.to_string())\n");
    code.push_str("}\n\n");

    Ok(code)
}

/// 辅助函数：根据Flex配置确定X轴计算函数
fn get_flex_x_calc_fn(flex_config: Option<&FlexConfig>) -> &'static str {
    match flex_config {
        Some(config) => match config.justify_content {
            JustifyContent::Center => "flex_center_x",
            JustifyContent::SpaceAround => "flex_space_around_x",
            JustifyContent::SpaceBetween => "flex_space_between_x",
            _ => "flex_row_x",
        },
        None => "flex_row_x",
    }
}

// ========== 对外暴露的构建函数 ==========
/// 布局处理器主函数（编译期入口）
pub fn build(config: &BuildConfig, _progress: &ProgressTracker) -> Result<()> {
    // 1. 验证HTML文件存在
    if !config.main_layout_path.exists() {
        return Err(anyhow!(
            "布局HTML文件不存在: {}",
            config.main_layout_path.display()
        ));
    }

    // 2. 解析HTML布局（编译期核心）
    let compile_const = parse_html_layout(&config.main_layout_path).context("解析HTML布局失败")?;

    // 3. 确保输出目录存在
    std::fs::create_dir_all(&config.output_dir)
        .with_context(|| format!("创建输出目录失败: {}", config.output_dir.display()))?;

    // 4. 生成运行时代码
    generate_runtime_code(&compile_const, &config.output_dir).context("生成运行时布局代码失败")?;

    Ok(())
}
