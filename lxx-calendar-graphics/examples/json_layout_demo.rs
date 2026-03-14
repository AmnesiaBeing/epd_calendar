//! JSON 布局渲染示例
//! 演示如何使用类似 inksight 项目的 JSON 格式来绘制内容

use lxx_calendar_graphics::{LayoutData, LayoutParser, Renderer};

fn main() {
    println!("=== lxx-calendar-graphics JSON 布局渲染示例 ===\n");

    // 示例 1: 简单的每日推荐布局
    let daily_layout = r#"{
        "status_bar": {
            "line_width": 1,
            "dashed": false
        },
        "body": [
            {
                "type": "two_column",
                "left_width": 116,
                "gap": 10,
                "left": [
                    {"type": "text", "field": "year", "font_size": 12, "align": "center", "margin_x": 8},
                    {"type": "big_number", "field": "day", "font_size": 46, "align": "center"},
                    {"type": "text", "field": "month_cn", "font_size": 14, "align": "center", "margin_x": 8},
                    {"type": "text", "field": "weekday_cn", "font_size": 12, "align": "center", "margin_x": 8}
                ],
                "right": [
                    {"type": "text", "field": "quote", "font_size": 14, "align": "left", "margin_x": 6, "max_lines": 4},
                    {"type": "text", "template": "— {author}", "font_size": 11, "align": "right", "margin_x": 6, "max_lines": 1},
                    {"type": "separator", "style": "dashed", "margin_x": 12},
                    {"type": "text", "field": "book_title", "font_size": 13, "align": "left", "margin_x": 6, "max_lines": 2},
                    {"type": "text", "field": "book_desc", "font_size": 11, "align": "left", "margin_x": 6, "max_lines": 2}
                ]
            }
        ],
        "footer": {
            "label": "DAILY",
            "attribution_template": "— InkSight"
        }
    }"#;

    let daily_data_json = r#"{
        "year": 2026,
        "day": 13,
        "month_cn": "三月",
        "weekday_cn": "周五",
        "quote": "阻碍行动的障碍，本身就是行动的路。",
        "author": "马可·奥勒留",
        "book_title": "《沉思录》",
        "book_desc": "罗马帝王的自省笔记。"
    }"#;

    println!("=== 每日推荐布局示例 ===");
    println!("布局 JSON:\n{}\n", daily_layout);
    println!("数据 JSON:\n{}\n", daily_data_json);

    // 解析数据
    let daily_data = LayoutParser::parse_data(daily_data_json).expect("解析数据失败");
    print_layout_data(&daily_data);

    // 示例 2: 天气看板布局
    let weather_layout = r#"{
        "body": [
            {"type": "text", "field": "city", "font_size": 16, "align": "left", "margin_x": 20},
            {"type": "spacer", "height": 8},
            {
                "type": "two_column",
                "left_width": 170,
                "gap": 6,
                "left": [
                    {"type": "big_number", "field": "today_temp", "font_size": 52, "align": "left", "unit": "°C"},
                    {"type": "spacer", "height": 2},
                    {"type": "weather_icon_text", "code_field": "today_code", "field": "today_desc", "font_size": 16, "icon_size": 24},
                    {"type": "text", "field": "today_range", "font_size": 14, "align": "left"}
                ],
                "right": [
                    {"type": "forecast_cards", "field": "forecast", "max_items": 4, "icon_size": 32}
                ]
            },
            {"type": "separator", "style": "solid", "margin_x": 20},
            {"type": "text", "field": "advice", "font_size": 12, "align": "center", "max_lines": 2}
        ]
    }"#;

    let weather_data_json = r#"{
        "city": "北京",
        "today_temp": 18,
        "today_desc": "晴",
        "today_code": 100,
        "today_range": "12° / 22°",
        "advice": "气温适宜，轻装出行",
        "forecast": [
            {"temp": "16°", "condition": "多云"},
            {"temp": "19°", "condition": "晴"},
            {"temp": "15°", "condition": "小雨"}
        ]
    }"#;

    println!("\n=== 天气看板布局示例 ===");
    println!("布局 JSON:\n{}\n", weather_layout);
    println!("数据 JSON:\n{}\n", weather_data_json);

    // 解析数据
    let weather_data = LayoutParser::parse_data(weather_data_json).expect("解析数据失败");
    print_layout_data(&weather_data);

    // 示例 3: 使用 Rust API 直接构建数据（no_std 方式）
    println!("\n=== no_std 方式构建数据示例 ===");
    let mut data = LayoutData::new();
    let _ = data.set_string("day", "13");
    let _ = data.set_string("month_cn", "三月");
    let _ = data.set_string("weekday_cn", "周五");
    let _ = data.set_i64("year", 2026);
    
    print_layout_data(&data);

    println!("\n=== 支持的布局块类型 ===");
    println!("- text: 文本块");
    println!("- big_number: 大号数字");
    println!("- separator: 分隔线");
    println!("- spacer: 间距");
    println!("- two_column: 两列布局");
    println!("- icon_text: 图标 + 文本");
    println!("- weather_icon_text: 天气图标 + 文本");
    println!("- progress_bar: 进度条");
    println!("- image: 图片");
    println!("- forecast_cards: 预报卡片");
    println!("- centered_text: 居中文本");
    println!("- vertical_stack: 垂直堆叠");
    println!("- conditional: 条件块");
    println!("- section: 区块");
    println!("- list: 列表");
    println!("- icon_list: 图标列表");
    println!("- key_value: 键值对");
    println!("- group: 组");

    println!("\n=== 使用方式 ===");
    println!(r#"
// std 环境（桌面测试）
use lxx_calendar_graphics::{Renderer, LayoutData, LayoutParser};

let mut renderer = Renderer::<384000>::new(800, 480);

// 解析 JSON 布局
let layout_json = r#"{{
    "body": [
        {{"type": "big_number", "field": "day", "font_size": 72}},
        {{"type": "text", "field": "month_cn", "font_size": 18}}
    ]
}}"#;

// 解析数据
let data = LayoutParser::parse_data(r#"{{"day": 13, "month_cn": "三月"}}"#)?;

// 渲染
renderer.render_from_json(layout_json, &data)?;

// 获取帧缓冲区数据用于显示
let framebuffer = renderer.framebuffer();
// ... 发送到墨水屏显示


// no_std 环境（嵌入式）
use lxx_calendar_graphics::{Renderer, LayoutData, layout::*};

let mut renderer = Renderer::<384000>::new(800, 480);

// 构建数据
let mut data = LayoutData::new();
data.set_string("day", "13").ok();
data.set_string("month_cn", "三月").ok();

// 定义布局（在编译时定义）
let layout = LayoutDefinition {{
    body: vec![
        Block::BigNumber(BigNumberBlock {{
            field: "day".to_string(),
            font_size: 72,
            ..Default::default()
        }}),
    ]),
    ..Default::default()
}};

// 渲染
renderer.render_layout(&layout, &data)?;
    "#);
}

fn print_layout_data(data: &LayoutData) {
    println!("解析后的数据:");
    for field in data.get_fields() {
        if let Some(value) = data.get_string(field) {
            println!("  {}: {}", field, value);
        }
    }
}
