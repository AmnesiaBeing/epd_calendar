# JSON 布局系统使用指南

## 概述

JSON 布局系统允许你通过 JSON 配置文件定义墨水屏的显示内容和布局，无需修改代码即可添加新的显示模式。

## 快速开始

### 1. 基本使用示例

```rust
use lxx_calendar_graphics::layout::{ModeLoader, JsonLayoutRenderer};
use alloc::collections::BTreeMap;

// 创建模式加载器
let mut loader = ModeLoader::new();

// 加载模式定义
let mode_json = r#"{
    "mode_id": "POETRY",
    "display_name": "每日诗词",
    "layout": {
        "status_bar": { "show_date": true, "show_weather": true },
        "body": {
            "blocks": [
                {
                    "type": "section",
                    "title": "📖 今日诗词",
                    "children": [
                        { "type": "text", "field": "poetry_title", "font_size": 18 },
                        { "type": "text", "field": "poetry_content", "font_size": 14 }
                    ]
                }
            ]
        },
        "footer": { "label": "POETRY" }
    }
}"#;

loader.load_from_json(mode_json).unwrap();

// 准备数据
let mut data = BTreeMap::new();
data.insert("poetry_title".to_string(), "静夜思".to_string());
data.insert("poetry_content".to_string(), "床前明月光，疑是地上霜".to_string());
data.insert("date_str".to_string(), "2026-01-15".to_string());
data.insert("weather_str".to_string(), "晴 15°C".to_string());

// 渲染
let renderer = JsonLayoutRenderer::new();
let mode = loader.get_mode("POETRY").unwrap();
renderer.render(&mut framebuffer, &mode.layout, &data, "POETRY").unwrap();
```

### 2. 从 Flash 加载模式

```rust
use lxx_calendar_graphics::layout::ModeLoader;

let mut loader = ModeLoader::new();

// 从 Flash 加载用户自定义模式
let count = loader.load_from_flash(&mut flash_device)?;

// 如果 Flash 中没有模式，加载内置模式
if count == 0 {
    loader.load_builtin_modes()?;
}

// 保存模式到 Flash
loader.save_to_flash(&mut flash_device)?;
```

## 布局块类型

### Text - 文本块

最基本的显示单元，用于显示文本内容。

```json
{
  "type": "text",
  "field": "poetry_title",
  "font_size": 18,
  "align": "center",
  "max_lines": 2,
  "margin_x": 10,
  "template": "标题：{poetry_title}"
}
```

**属性说明：**
- `field`: 数据字段名，从数据上下文中获取
- `font_size`: 字体大小（像素）
- `align`: 对齐方式（left/center/right）
- `max_lines`: 最大行数，超出截断并添加省略号
- `margin_x`: 水平边距（像素）
- `template`: 可选的模板字符串，支持 `{field}` 占位符

### Icon - 图标

显示图标（目前为占位符，待实现完整图标系统）。

```json
{
  "type": "icon",
  "name": "book",
  "size": 24
}
```

### Separator - 分隔线

绘制分隔线。

```json
{
  "type": "separator",
  "style": "solid",
  "line_width": 1,
  "margin_x": 10,
  "width": 60
}
```

**样式说明：**
- `solid`: 实线
- `dashed`: 虚线
- `dotted`: 点线
- `short`: 短线（居中显示）

### Spacer - 间距

添加垂直空白间距。

```json
{
  "type": "spacer",
  "height": 16
}
```

### Section - 区块

带标题的区块，可包含子块。

```json
{
  "type": "section",
  "title": "今日诗词",
  "icon": "book",
  "children": [
    { "type": "text", "field": "title" },
    { "type": "text", "field": "content" }
  ]
}
```

### VStack - 垂直堆叠

垂直堆叠多个子块。

```json
{
  "type": "vstack",
  "spacing": 8,
  "children": [
    { "type": "text", "field": "line1" },
    { "type": "text", "field": "line2" }
  ]
}
```

### Conditional - 条件渲染

根据条件决定是否渲染。

```json
{
  "type": "conditional",
  "field": "solar_term",
  "condition": { "op": "exists" },
  "then_children": [
    { "type": "text", "field": "solar_term" }
  ],
  "else_children": [
    { "type": "text", "field": "default_text" }
  ]
}
```

**条件类型：**
- `exists`: 字段存在且非空
- `eq`: 等于指定值
- `not_eq`: 不等于指定值
- `gt`: 大于指定值（数值）
- `lt`: 小于指定值（数值）
- `gte`: 大于等于
- `lte`: 小于等于

### BigNumber - 大号数字

显示大号数字，可带单位。

```json
{
  "type": "big_number",
  "field": "temp",
  "font_size": 56,
  "align": "center",
  "unit": "°C"
}
```

### ProgressBar - 进度条

显示进度条。

```json
{
  "type": "progress_bar",
  "field": "day_of_year",
  "max_field": "days_in_year",
  "width": 100,
  "height": 8,
  "margin_x": 20
}
```

## 完整布局结构

```json
{
  "mode_id": "MODE_ID",
  "display_name": "显示名称",
  "icon": "图标名称",
  "cacheable": true,
  "layout": {
    "status_bar": {
      "show_date": true,
      "show_weather": true,
      "show_battery": true,
      "show_time": false,
      "line_width": 1,
      "dashed": false
    },
    "body": {
      "blocks": [...],
      "align": "center",
      "vertical_align": "center"
    },
    "footer": {
      "label": "MODE_ID",
      "show_mode_name": true,
      "line_width": 1,
      "dashed": false,
      "height": 30
    }
  }
}
```

## 数据字段

渲染时需要提供数据上下文。常用字段包括：

| 字段名 | 说明 | 示例 |
|--------|------|------|
| `date_str` | 日期字符串 | "2026-01-15 周四" |
| `weather_str` | 天气描述 | "晴 15°C" |
| `battery_pct` | 电池百分比 | "85%" |
| `poetry_title` | 诗词标题 | "静夜思" |
| `poetry_author` | 诗词作者 | "李白" |
| `poetry_content` | 诗词内容 | "床前明月光..." |
| `quote_text` | 一言文本 | "生活就像一盒巧克力..." |
| `quote_from` | 一言来源 | "《阿甘正传》" |
| `temp` | 温度 | "25" |
| `humidity` | 湿度 | "65" |
| `lunar_month` | 农历月份 | "腊月" |
| `lunar_day` | 农历日期 | "十五" |
| `solar_term` | 节气 | "立春" |
| `festival` | 节日 | "春节" |

## 内置模式

系统预置了以下模式：

1. **POETRY** - 每日诗词
2. **QUOTE** - 每日一言
3. **DATE** - 日历
4. **WEATHER** - 天气

## 自定义模式

### 方法 1: 直接加载 JSON

```rust
let mut loader = ModeLoader::new();
loader.load_from_json(custom_mode_json)?;
```

### 方法 2: 存储到 Flash

```rust
// 加载后保存到 Flash
loader.save_to_flash(&mut flash_device)?;

// 下次启动时从 Flash 加载
loader.load_from_flash(&mut flash_device)?;
```

## 最佳实践

### 1. 内存优化

- 模式定义存储在 Flash 中，不占用 RAM
- 仅在渲染时解析需要的字段
- 使用 `heapless::Vec` 限制最大块数量

### 2. 性能优化

- 常用模式预加载到内存
- 使用 `cacheable: true` 启用缓存
- 避免过多的条件判断块

### 3. 布局设计

- 保持布局简洁，避免过深的嵌套
- 使用 `vertical_align: "center"` 实现垂直居中
- 为不同屏幕尺寸设计不同的布局

## 故障排除

### 模式加载失败

检查 JSON 格式是否正确，字段名是否匹配。

### 文本不显示

确认数据上下文中包含对应的字段。

### 布局错乱

检查 `font_size` 和 `margin_x` 设置是否合理。

## 后续计划

- [ ] 完整图标系统
- [ ] TrueType 字体渲染
- [ ] 更多布局块类型（图片、二维码等）
- [ ] 布局预览工具
