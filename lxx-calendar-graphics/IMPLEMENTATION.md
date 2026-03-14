# lxx-calendar-graphics JSON 布局渲染实现

## 概述

本实现参考 inksight 项目的 JSON 布局格式，为墨水屏日历提供了一个声明式的 UI 布局系统。

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    应用层 (Application)                      │
├─────────────────────────────────────────────────────────────┤
│  std 模式：                        no_std 模式：              │
│  - JSON 解析 (serde_json)          - 直接构建 LayoutData     │
│  - LayoutParser::parse_data()      - LayoutData::new()      │
├─────────────────────────────────────────────────────────────┤
│                  布局引擎 (LayoutEngine)                     │
│  - 解析 LayoutDefinition                                    │
│  - 遍历 Block 树                                             │
│  - 调用 TextRenderer/IconRenderer                           │
├─────────────────────────────────────────────────────────────┤
│  渲染器 (Renderers)                                          │
│  - TextRenderer: 文本渲染（多字体大小）                       │
│  - IconRenderer: 图标渲染（预渲染位图）                       │
│  - LayoutRenderer: 布局元素（分隔线等）                       │
├─────────────────────────────────────────────────────────────┤
│              帧缓冲区 (Framebuffer<SIZE>)                    │
│              - 单色 1-bit per pixel                          │
│              - 直接映射到墨水屏                              │
└─────────────────────────────────────────────────────────────┘
```

## 核心组件

### 1. 布局定义 (`lxx-calendar-common/src/types/layout.rs`)

```rust
pub struct LayoutDefinition {
    pub status_bar: Option<StatusBarConfig>,
    pub body: Vec<Block>,
    pub footer: Option<FooterConfig>,
    pub layout_overrides: Option<LayoutOverrides>,
}

// 支持的块类型
pub enum Block {
    Text(TextBlock),
    BigNumber(BigNumberBlock),
    Separator(SeparatorBlock),
    Spacer(SpacerBlock),
    TwoColumn(TwoColumnBlock),
    IconText(IconTextBlock),
    WeatherIconText(WeatherIconTextBlock),
    ProgressBar(ProgressBarBlock),
    // ... 更多类型
}
```

### 2. 布局数据 (`lxx-calendar-graphics/src/renderer/layout_engine.rs`)

```rust
/// no_std 兼容的数据结构
pub struct LayoutData {
    fields: heapless::HashMap<heapless::String<32>, LayoutValue, 32>,
}

pub enum LayoutValue {
    String(heapless::String<64>),
    I64(i64),
    F64(f64),
    Bool(bool),
    Array(heapless::Vec<LayoutData, 16>),
}
```

### 3. JSON 解析器 (`lxx-calendar-graphics/src/parser/layout_parser.rs`)

```rust
/// 仅在 std 模式下可用
#[cfg(feature = "std")]
impl LayoutParser {
    pub fn parse_layout(json_str: &str) -> Result<LayoutDefinition, ParseError>;
    pub fn parse_data(json_str: &str) -> Result<LayoutData, ParseError>;
}
```

## 使用示例

### std 环境（桌面测试/服务端渲染）

```rust
use lxx_calendar_graphics::{Renderer, LayoutData, LayoutParser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建渲染器
    let mut renderer = Renderer::<384000>::new(800, 480);
    
    // JSON 布局定义
    let layout_json = r#"{
        "status_bar": {"line_width": 1, "dashed": false},
        "body": [
            {"type": "text", "field": "city", "font_size": 16, "margin_x": 20},
            {"type": "spacer", "height": 8},
            {
                "type": "two_column",
                "left_width": 170,
                "gap": 6,
                "left": [
                    {"type": "big_number", "field": "today_temp", "font_size": 52, "unit": "°C"},
                    {"type": "weather_icon_text", "code_field": "today_code", "field": "today_desc"}
                ],
                "right": [
                    {"type": "forecast_cards", "field": "forecast", "max_items": 4}
                ]
            },
            {"type": "separator", "style": "solid", "margin_x": 20},
            {"type": "text", "field": "advice", "font_size": 12, "align": "center"}
        ],
        "footer": {
            "label": "WEATHER",
            "attribution_template": "— InkSight"
        }
    }"#;
    
    // 解析数据
    let data = LayoutParser::parse_data(r#"{
        "city": "北京",
        "today_temp": 18,
        "today_desc": "晴",
        "today_code": 100,
        "today_range": "12° / 22°",
        "advice": "气温适宜，轻装出行",
        "forecast": [
            {"temp": "16°", "condition": "多云"},
            {"temp": "19°", "condition": "晴"}
        ]
    }"#)?;
    
    // 渲染
    renderer.render_from_json(layout_json, &data)?;
    
    // 获取帧缓冲区数据
    let framebuffer = renderer.framebuffer();
    let buffer = framebuffer.buffer();
    
    // 发送到墨水屏显示
    // display.show(buffer)?;
    
    Ok(())
}
```

### no_std 环境（嵌入式）

```rust
#![no_std]

use lxx_calendar_graphics::{Renderer, LayoutData, layout::*};
use heapless::Vec;

pub fn render_calendar_display(
    renderer: &mut Renderer<384000>,
    day: u16,
    month: &str,
    quote: &str,
) -> Result<(), SystemError> {
    // 构建数据（无 JSON 解析）
    let mut data = LayoutData::new();
    data.set_string("day", day.to_string().as_str()).map_err(|_| SystemError::InvalidState)?;
    data.set_string("month_cn", month).map_err(|_| SystemError::InvalidState)?;
    data.set_string("quote", quote).map_err(|_| SystemError::InvalidState)?;
    
    // 定义布局（编译时定义）
    let layout = LayoutDefinition {
        body: vec![
            Block::BigNumber(BigNumberBlock {
                field: "day".to_string(),
                font_size: 72,
                align: TextAlign::Center,
                margin_x: 0,
                unit: None,
            }),
            Block::Text(TextBlock {
                field: Some("month_cn".to_string()),
                template: None,
                font: "noto_serif_regular".to_string(),
                font_size: 18,
                align: TextAlign::Center,
                margin_x: 24,
                max_lines: 1,
            }),
            Block::Separator(SeparatorBlock {
                style: SeparatorStyle::Dashed,
                margin_x: 24,
                width: None,
                line_width: 1,
            }),
            Block::Text(TextBlock {
                field: Some("quote".to_string()),
                template: None,
                font: "noto_serif_regular".to_string(),
                font_size: 14,
                align: TextAlign::Center,
                margin_x: 24,
                max_lines: 3,
            }),
        ],
        status_bar: None,
        footer: None,
        layout_overrides: None,
    };
    
    // 渲染
    renderer.render_layout(&layout, &data)?;
    
    Ok(())
}
```

## 支持的布局块

| 块类型 | 描述 | 示例 |
|--------|------|------|
| `text` | 文本块 | `{"type": "text", "field": "title", "font_size": 14}` |
| `big_number` | 大号数字 | `{"type": "big_number", "field": "day", "font_size": 72}` |
| `separator` | 分隔线 | `{"type": "separator", "style": "dashed"}` |
| `spacer` | 间距 | `{"type": "spacer", "height": 8}` |
| `two_column` | 两列布局 | `{"type": "two_column", "left_width": 170, "left": [...], "right": [...]}` |
| `icon_text` | 图标 + 文本 | `{"type": "icon_text", "icon": "sunny", "text": "晴"}` |
| `weather_icon_text` | 天气图标 + 文本 | `{"type": "weather_icon_text", "code_field": "code", "field": "desc"}` |
| `progress_bar` | 进度条 | `{"type": "progress_bar", "field": "progress", "max_field": "max"}` |
| `forecast_cards` | 预报卡片 | `{"type": "forecast_cards", "field": "forecast", "max_items": 4}` |
| `centered_text` | 居中文本 | `{"type": "centered_text", "field": "title"}` |
| `vertical_stack` | 垂直堆叠 | `{"type": "vertical_stack", "children": [...]}` |
| `conditional` | 条件块 | `{"type": "conditional", "field": "status", "conditions": [...]}` |
| `list` | 列表 | `{"type": "list", "field": "items", "item_template": "{index}. {name}"}` |

## 特性

- **no_std 兼容**: 核心渲染引擎可在无标准库环境下运行
- **可选 JSON 解析**: `std` 特性启用 JSON 解析功能
- **inksight 兼容**: 布局格式参考 inksight 项目
- **灵活的数据绑定**: 支持 field、template 等多种数据绑定方式
- **响应式布局**: 支持不同屏幕尺寸的重写配置

## 文件结构

```
lxx-calendar-graphics/
├── src/
│   ├── renderer/
│   │   ├── framebuffer.rs    # 帧缓冲区管理
│   │   ├── text.rs           # 文本渲染器
│   │   ├── icon.rs           # 图标渲染器
│   │   ├── layout.rs         # 布局渲染器
│   │   ├── layout_engine.rs  # 布局引擎（新增）
│   │   └── mod.rs
│   ├── parser/
│   │   ├── layout_parser.rs  # JSON 解析器（std only）
│   │   └── mod.rs
│   ├── assets/               # 生成的资源文件
│   └── lib.rs
├── builder/                  # 构建时资源生成
│   └── modules/
│       ├── font_generator.rs
│       └── icon_generator.rs
├── examples/
│   └── json_layout_demo.rs   # 示例代码
└── Cargo.toml
```

## 与 inksight 的对比

| 特性 | inksight | lxx-calendar-graphics |
|------|----------|----------------------|
| 目标平台 | Web/桌面 | 嵌入式墨水屏 |
| 运行环境 | Node.js | Rust (no_std) |
| JSON 解析 | 运行时 | 编译时/外部解析 |
| 数据结构 | JavaScript Object | LayoutData (heapless) |
| 渲染输出 | HTML/CSS | 帧缓冲区 (1-bit) |
| 字体 | Web Fonts | 位图字体（预生成） |
| 图标 | SVG | 预渲染位图 |

## 下一步

1. 集成真实字体数据（build.rs 生成）
2. 集成真实图标数据（SVG 预渲染）
3. 添加更多布局块类型支持
4. 实现缓存机制提高性能
5. 添加动画/过渡效果支持
