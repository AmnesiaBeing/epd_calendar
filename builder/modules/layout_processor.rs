//! HTML布局解析器
//! 解析800x480信息面板HTML，提取布局元素的坐标、尺寸、样式、数据源映射
//! 生成Rust布局规则文件，供渲染引擎直接调用

#![allow(unused)]

use anyhow::{Context, Result, anyhow};
use html_parser::{Dom, Element, Node};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::builder::utils::file_utils;

/// 布局元素类型
#[derive(Debug, Clone)]
pub enum LayoutElementType {
    Text,      // 文本元素
    Icon,      // 图标元素
    Line,      // 线条元素（水平/垂直分割线）
    Container, // 容器元素（仅用于布局计算，不渲染）
}

/// 文本对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// 布局元素样式
#[derive(Debug, Clone)]
pub struct LayoutStyle {
    // 通用样式
    pub x: u32,                    // 元素X坐标（相对父容器，最终转为绝对坐标）
    pub y: u32,                    // 元素Y坐标
    pub width: u32,                // 元素宽度
    pub height: u32,               // 元素高度
    pub parent_id: Option<String>, // 父容器ID

    // 文本样式
    pub font_size: Option<u16>,        // 字体尺寸（px）
    pub text_align: Option<TextAlign>, // 文本对齐

    // 图标样式
    pub icon_id_pattern: Option<String>, // 图标ID匹配模式（如time_digit/digit_{}）
}

/// 布局元素定义
#[derive(Debug, Clone)]
pub struct LayoutElement {
    pub id: String,                      // 元素ID（与HTML id一致）
    pub element_type: LayoutElementType, // 元素类型
    pub style: LayoutStyle,              // 样式属性
    pub data_key: Option<String>,        // 数据源缓存key（如time.hour、weather.location）
    pub placeholder: Option<String>,     // HTML占位符（如{{date}}）
}

/// 布局规则（所有元素的集合）
#[derive(Debug, Clone)]
pub struct LayoutRules {
    pub elements: HashMap<String, LayoutElement>, // 按ID索引的元素
    pub root_size: (u32, u32),                    // 根容器尺寸（800x480）
}

/// 解析HTML文件生成布局规则
pub fn parse_html_layout(html_path: &Path) -> Result<LayoutRules> {
    // 1. 读取并解析HTML
    let html_content = fs::read_to_string(html_path)
        .with_context(|| format!("读取HTML文件失败: {}", html_path.display()))?;
    let dom = Dom::parse(&html_content).with_context(|| "解析HTML失败")?;

    // 2. 找到根容器（#root_container）
    let root_element = find_element_by_id(&dom, "root_container")
        .ok_or_else(|| anyhow!("未找到根容器#root_container"))?;

    // 3. 初始化布局规则（根尺寸800x480）
    let mut rules = LayoutRules {
        elements: HashMap::new(),
        root_size: (800, 480),
    };

    // 4. 解析根容器下的所有子元素
    parse_child_elements(&root_element, &mut rules, None, (0, 0))?;

    Ok(rules)
}

/// 递归解析子元素
fn parse_child_elements(
    parent: &Element,
    rules: &mut LayoutRules,
    parent_id: Option<String>,
    parent_origin: (u32, u32), // 父容器原点坐标
) -> Result<()> {
    for node in &parent.children {
        match node {
            Node::Element(child) => {
                let element_id = child.attributes.get("id").cloned().unwrap_or_default();
                if element_id.is_empty() {
                    // 跳过无ID的元素
                    parse_child_elements(child, rules, parent_id.clone(), parent_origin)?;
                    continue;
                }

                // 1. 解析元素样式和位置
                let style = parse_element_style(child, parent_origin, parent, rules)?;

                // 2. 解析元素类型
                let element_type = parse_element_type(child);

                // 3. 解析数据源映射（占位符→缓存key）
                let (data_key, placeholder) = parse_data_mapping(child);

                // 4. 创建布局元素
                let layout_element = LayoutElement {
                    id: element_id.clone(),
                    element_type,
                    style: LayoutStyle {
                        parent_id: parent_id.clone(),
                        ..style
                    },
                    data_key,
                    placeholder,
                };

                // 5. 添加到布局规则
                rules.elements.insert(element_id.clone(), layout_element);

                // 6. 递归解析子元素
                let child_origin = (style.x, style.y);
                parse_child_elements(child, rules, Some(element_id), child_origin)?;
            }
            _ => continue,
        }
    }

    Ok(())
}

/// 解析元素样式（坐标、尺寸、字体等）
fn parse_element_style(
    element: &Element,
    parent_origin: (u32, u32),
    parent: &Element,
    rules: &LayoutRules,
) -> Result<LayoutStyle> {
    let id = element.attributes.get("id").cloned().unwrap_or_default();
    let mut style = LayoutStyle {
        x: parent_origin.0,
        y: parent_origin.1,
        width: 0,
        height: 0,
        parent_id: None,
        font_size: None,
        text_align: None,
        icon_id_pattern: None,
    };

    // 固定尺寸元素解析（基于HTML的800x480设计）
    match id.as_str() {
        // 根容器
        "root_container" => {
            style.width = 800;
            style.height = 480;
            style.x = 0;
            style.y = 0;
        }

        // 时间模块
        "time_wrap" => {
            style.width = 800 - 20; // 减去padding
            style.height = 60;
            style.x = 10;
            style.y = 10;
        }
        "time_digit_hour_tens"
        | "time_digit_hour_ones"
        | "time_digit_minute_tens"
        | "time_digit_minute_ones" => {
            style.width = 40;
            style.height = 60;
            style.icon_id_pattern = Some(format!(
                "time_digit/digit_{}",
                id.split('_').last().unwrap()
            ));

            // 时间数字位置计算
            let base_x = 800 / 2 - 100; // 居中偏移
            match id.as_str() {
                "time_digit_hour_tens" => style.x = base_x,
                "time_digit_hour_ones" => style.x = base_x + 50,
                "time_digit_minute_tens" => style.x = base_x + 120,
                "time_digit_minute_ones" => style.x = base_x + 170,
                _ => (),
            }
            style.y = 10;
        }
        "time_digit" => {
            // 冒号
            style.width = 20;
            style.height = 60;
            style.x = base_x + 90;
            style.y = 10;
            style.icon_id_pattern = Some("time_digit/digit_colon".to_string());
        }

        // 日期模块
        "date_wrap" => {
            style.width = 800 - 20;
            style.height = 30;
            style.x = 10;
            style.y = 80;
            style.font_size = Some(24);
            style.text_align = Some(TextAlign::Center);
        }

        // 水平分割线
        "divider1" => {
            style.width = 800 - 20;
            style.height = 1;
            style.x = 10;
            style.y = 120;
        }
        "divider2" => {
            style.width = 800 - 20;
            style.height = 1;
            style.x = 10;
            style.y = 400;
        }

        // 垂直分割线
        "vertical_divider" => {
            style.width = 1;
            style.height = 250;
            style.x = 400;
            style.y = 130;
        }

        // 农历模块
        "lunar_wrap" => {
            style.width = 400 - 20;
            style.height = 250;
            style.x = 10;
            style.y = 130;
        }
        "lunar_year" => {
            style.width = 400 - 40;
            style.height = 30;
            style.x = 30;
            style.y = 130;
            style.font_size = Some(24);
            style.text_align = Some(TextAlign::Center);
        }
        "lunar_day" => {
            style.width = 400 - 40;
            style.height = 50;
            style.x = 30;
            style.y = 170;
            style.font_size = Some(40);
            style.text_align = Some(TextAlign::Center);
        }
        "lunar_yi_ji" => {
            style.width = 400 - 40;
            style.height = 150;
            style.x = 30;
            style.y = 230;
            style.font_size = Some(16);
            style.text_align = Some(TextAlign::Left);
        }

        // 天气模块
        "weather_wrap" => {
            style.width = 400 - 20;
            style.height = 250;
            style.x = 410;
            style.y = 130;
        }
        "weather_location" => {
            style.width = 200;
            style.height = 20;
            style.x = 430;
            style.y = 130;
            style.font_size = Some(16);
            style.text_align = Some(TextAlign::Left);
        }
        "weather_temp_hum" => {
            style.width = 200;
            style.height = 20;
            style.x = 600;
            style.y = 130;
            style.font_size = Some(16);
            style.text_align = Some(TextAlign::Right);
        }
        "weather_day1" | "weather_day2" | "weather_day3" => {
            style.width = 120;
            style.height = 200;
            style.x = match id.as_str() {
                "weather_day1" => 430,
                "weather_day2" => 560,
                "weather_day3" => 690,
                _ => 430,
            };
            style.y = 160;
            style.font_size = Some(16);
            style.text_align = Some(TextAlign::Center);
        }
        "weather_icon1" | "weather_icon2" | "weather_icon3" => {
            style.width = 40;
            style.height = 40;
            style.x = match id.as_str() {
                "weather_icon1" => 460,
                "weather_icon2" => 590,
                "weather_icon3" => 720,
                _ => 460,
            };
            style.y = 200;
            style.icon_id_pattern = Some("icon_{}".to_string());
        }

        // 格言模块
        "motto_wrap" => {
            style.width = 800 - 40;
            style.height = 80;
            style.x = 20;
            style.y = 410;
        }
        "motto_content" => {
            style.width = 800 - 40;
            style.height = 60;
            style.x = 20;
            style.y = 410;
            style.font_size = Some(24);
            style.text_align = Some(TextAlign::Center); // 动态调整
        }
        "motto_source" => {
            style.width = 800 - 40;
            style.height = 20;
            style.x = 20;
            style.y = 470;
            style.font_size = Some(16);
            style.text_align = Some(TextAlign::Right);
        }

        // 状态图标
        "network_icon" => {
            style.width = 32;
            style.height = 32;
            style.x = 10;
            style.y = 10;
            style.icon_id_pattern = Some("network/{}".to_string());
        }
        "battery_icon" => {
            style.width = 32;
            style.height = 32;
            style.x = 758;
            style.y = 10;
            style.icon_id_pattern = Some("battery/battery-{}".to_string());
        }
        "charging_icon" => {
            style.width = 32;
            style.height = 32;
            style.x = 718;
            style.y = 10;
            style.icon_id_pattern = Some("battery/bolt".to_string());
        }

        // 其他元素
        _ => {
            // 默认尺寸
            style.width = 100;
            style.height = 30;
        }
    }

    Ok(style)
}

/// 解析元素类型
fn parse_element_type(element: &Element) -> LayoutElementType {
    let id = element.attributes.get("id").cloned().unwrap_or_default();

    if id.contains("digit") || id.contains("icon") {
        LayoutElementType::Icon
    } else if id.contains("divider") {
        LayoutElementType::Line
    } else if id.contains("wrap") {
        LayoutElementType::Container
    } else {
        LayoutElementType::Text
    }
}

/// 解析数据源映射（占位符→缓存key）
fn parse_data_mapping(element: &Element) -> (Option<String>, Option<String>) {
    let id = element.attributes.get("id").cloned().unwrap_or_default();
    let mut placeholder = None;
    let mut data_key = None;

    // 提取文本内容中的占位符
    for node in &element.children {
        if let Node::Text(text) = node {
            if text.contains("{{") && text.contains("}}") {
                placeholder = Some(text.trim().to_string());
                break;
            }
        }
    }

    // 映射占位符到缓存key
    if let Some(ph) = &placeholder {
        data_key = match ph.as_str() {
            "{{date}}" => Some("date.full".to_string()),
            "{{lunar_year}}" => Some("lunar.year".to_string()),
            "{{lunar_day}}" => Some("lunar.day".to_string()),
            "{{lunar_suitable}}" => Some("lunar.suitable".to_string()),
            "{{lunar_avoid}}" => Some("lunar.avoid".to_string()),
            "{{weather_location}}" => Some("weather.location".to_string()),
            "{{weather_temp_hum}}" => Some("weather.temp_hum".to_string()),
            "{{motto_content}}" => Some("motto.content".to_string()),
            "{{motto_source}}" => Some("motto.source".to_string()),
            "{{day1}}" => Some("weather.day1.name".to_string()),
            "{{day2}}" => Some("weather.day2.name".to_string()),
            "{{day3}}" => Some("weather.day3.name".to_string()),
            "{{desc1}}" => Some("weather.day1.desc".to_string()),
            "{{desc2}}" => Some("weather.day2.desc".to_string()),
            "{{desc3}}" => Some("weather.day3.desc".to_string()),
            _ => None,
        };
    }

    // 特殊处理时间/图标元素的data_key
    if id.contains("time_digit") {
        data_key = Some(format!("time.{}", id.split('_').last().unwrap()));
    } else if id.contains("weather_icon") {
        let day = id.chars().last().unwrap();
        data_key = Some(format!("weather.day{}.icon", day));
    } else if id == "network_icon" {
        data_key = Some("system.network".to_string());
    } else if id == "battery_icon" {
        data_key = Some("system.battery".to_string());
    } else if id == "charging_icon" {
        data_key = Some("system.charging".to_string());
    }

    (data_key, placeholder)
}

/// 根据ID查找元素
fn find_element_by_id(dom: &Dom, id: &str) -> Option<&Element> {
    fn search_nodes(nodes: &[Node], id: &str) -> Option<&Element> {
        for node in nodes {
            if let Node::Element(element) = node {
                if element.attributes.get("id") == Some(&id.to_string()) {
                    return Some(element);
                }
                if let Some(found) = search_nodes(&element.children, id) {
                    return Some(found);
                }
            }
        }
        None
    }

    search_nodes(&dom.children, id)
}

/// 生成布局规则的Rust代码
pub fn generate_layout_rs(layout_rules: &LayoutRules, output_path: &Path) -> Result<()> {
    let mut content = String::new();

    // 头部注释
    content.push_str("//! 自动生成的布局规则文件\n");
    content.push_str("//! 由layout_processor从HTML解析生成，不要手动修改\n\n");
    content.push_str("#![allow(dead_code)]\n\n");
    content.push_str("use crate::kernel::render::layout::{LayoutElement, LayoutElementType, LayoutStyle, TextAlign};\n");
    content.push_str("use alloc::collections::HashMap;\n\n");

    // 定义TextAlign枚举
    content.push_str("// ==================== 对齐方式枚举 ====================\n");
    content.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    content.push_str("pub enum TextAlign {\n");
    content.push_str("    Left,\n");
    content.push_str("    Center,\n");
    content.push_str("    Right,\n");
    content.push_str("}\n\n");

    // 定义LayoutElementType枚举
    content.push_str("// ==================== 元素类型枚举 ====================\n");
    content.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    content.push_str("pub enum LayoutElementType {\n");
    content.push_str("    Text,\n");
    content.push_str("    Icon,\n");
    content.push_str("    Line,\n");
    content.push_str("    Container,\n");
    content.push_str("}\n\n");

    // 定义LayoutStyle结构体
    content.push_str("// ==================== 样式结构体 ====================\n");
    content.push_str("#[derive(Debug, Clone)]\n");
    content.push_str("pub struct LayoutStyle {\n");
    content.push_str("    pub x: u32,\n");
    content.push_str("    pub y: u32,\n");
    content.push_str("    pub width: u32,\n");
    content.push_str("    pub height: u32,\n");
    content.push_str("    pub parent_id: Option<&'static str>,\n");
    content.push_str("    pub font_size: Option<u16>,\n");
    content.push_str("    pub text_align: Option<TextAlign>,\n");
    content.push_str("    pub icon_id_pattern: Option<&'static str>,\n");
    content.push_str("}\n\n");

    // 定义LayoutElement结构体
    content.push_str("// ==================== 布局元素结构体 ====================\n");
    content.push_str("#[derive(Debug, Clone)]\n");
    content.push_str("pub struct LayoutElement {\n");
    content.push_str("    pub id: &'static str,\n");
    content.push_str("    pub element_type: LayoutElementType,\n");
    content.push_str("    pub style: LayoutStyle,\n");
    content.push_str("    pub data_key: Option<&'static str>,\n");
    content.push_str("}\n\n");

    // 生成布局规则常量
    content.push_str("// ==================== 布局规则常量 ====================\n");
    content.push_str("/// 800x480信息面板布局规则\n");
    content.push_str("pub const LAYOUT_RULES: LayoutRules = LayoutRules {\n");
    content.push_str(&format!(
        "    root_size: ({}, {}),\n",
        layout_rules.root_size.0, layout_rules.root_size.1
    ));
    content.push_str("    elements: {\n");
    content.push_str("        let mut map = HashMap::new();\n");

    // 添加所有元素
    for (id, element) in &layout_rules.elements {
        content.push_str(&format!(
            "        map.insert(\"{}\", LayoutElement {{\n",
            id
        ));
        content.push_str(&format!("            id: \"{}\",\n", id));
        content.push_str(&format!(
            "            element_type: LayoutElementType::{:?},\n",
            element.element_type
        ));
        content.push_str("            style: LayoutStyle {\n");
        content.push_str(&format!("                x: {},\n", element.style.x));
        content.push_str(&format!("                y: {},\n", element.style.y));
        content.push_str(&format!(
            "                width: {},\n",
            element.style.width
        ));
        content.push_str(&format!(
            "                height: {},\n",
            element.style.height
        ));
        content.push_str(&format!(
            "                parent_id: {},\n",
            match &element.style.parent_id {
                Some(p) => format!("Some(\"{}\")", p),
                None => "None".to_string(),
            }
        ));
        content.push_str(&format!(
            "                font_size: {},\n",
            match element.style.font_size {
                Some(s) => format!("Some({})", s),
                None => "None".to_string(),
            }
        ));
        content.push_str(&format!(
            "                text_align: {},\n",
            match element.style.text_align {
                Some(a) => format!("Some(TextAlign::{:?})", a),
                None => "None".to_string(),
            }
        ));
        content.push_str(&format!(
            "                icon_id_pattern: {},\n",
            match &element.style.icon_id_pattern {
                Some(p) => format!("Some(\"{}\")", p),
                None => "None".to_string(),
            }
        ));
        content.push_str("            },\n");
        content.push_str(&format!(
            "            data_key: {},\n",
            match &element.data_key {
                Some(k) => format!("Some(\"{}\")", k),
                None => "None".to_string(),
            }
        ));
        content.push_str("        });\n");
    }

    content.push_str("        map\n");
    content.push_str("    },\n");
    content.push_str("};\n\n");

    // 定义LayoutRules结构体
    content.push_str("// ==================== 布局规则结构体 ====================\n");
    content.push_str("#[derive(Debug, Clone)]\n");
    content.push_str("pub struct LayoutRules {\n");
    content.push_str("    pub root_size: (u32, u32),\n");
    content.push_str("    pub elements: HashMap<&'static str, LayoutElement>,\n");
    content.push_str("}\n\n");

    // 辅助方法：根据ID获取元素
    content.push_str("impl LayoutRules {\n");
    content.push_str("    /// 根据ID获取布局元素\n");
    content.push_str("    pub fn get_element(&self, id: &str) -> Option<&LayoutElement> {\n");
    content.push_str("        self.elements.get(id)\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    // 写入文件
    file_utils::write_string_file(output_path, &content)
        .with_context(|| format!("写入布局规则文件失败: {}", output_path.display()))?;

    Ok(())
}

/// 构建布局规则（解析HTML并生成Rust文件）
pub fn build(html_path: &Path, output_path: &Path) -> Result<()> {
    log::info!("解析HTML布局: {}", html_path.display());
    let layout_rules = parse_html_layout(html_path)?;

    log::info!("生成布局规则文件: {}", output_path.display());
    generate_layout_rs(&layout_rules, output_path)?;

    Ok(())
}
