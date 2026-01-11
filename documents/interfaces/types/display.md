# 显示 (Display) 数据类型文档

## 概述

显示类型用于管理墨水屏显示缓冲区、显示区域和刷新状态。本项目使用YRD0750RYF665F60墨水屏，800×480分辨率，黑白2色。

## DisplayBuffer

### 结构定义

```rust
pub struct DisplayBuffer {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
    pub color_mode: ColorMode,
}
```

### 字段说明

- `width` - 显示宽度（像素）
- `height` - 显示高度（像素）
- `data` - 显示数据（字节缓冲区）
- `color_mode` - 颜色模式

### 使用示例

```rust
let buffer = DisplayBuffer {
    width: 800,
    height: 480,
    data: vec![0u8; 800 * 480 / 8],
    color_mode: ColorMode::Monochrome,
};
```

## DisplayRegion

### 结构定义

```rust
pub struct DisplayRegion {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub name: String,
}
```

### 字段说明

- `x` - 区域左上角X坐标
- `y` - 区域左上角Y坐标
- `width` - 区域宽度
- `height` - 区域高度
- `name` - 区域名称

### 使用示例

```rust
let region = DisplayRegion {
    x: 0,
    y: 0,
    width: 800,
    height: 96,
    name: "time_region".to_string(),
};
```

## RefreshState

### 结构定义

```rust
pub enum RefreshState {
    Idle,
    SendingData,
    Refreshing,
    Error(RefreshError),
}
```

### 变体说明

- `Idle` - 空闲状态，可接收新刷新请求
- `SendingData` - 发送数据状态，正在发送像素数据
- `Refreshing` - 刷新状态，等待BUSY信号
- `Error` - 错误状态，刷新失败

### 使用示例

```rust
match refresh_state {
    RefreshState::Idle => lxx_common::info!("Ready for refresh"),
    RefreshState::SendingData => lxx_common::info!("Sending data..."),
    RefreshState::Refreshing => lxx_common::info!("Refreshing..."),
    RefreshState::Error(err) => lxx_common::error!("Error: {:?}", err),
}
```

## DisplayLayout

### 结构定义

```rust
pub struct DisplayLayout {
    pub regions: Vec<DisplayRegion>,
    pub font_size: u8,
    pub line_spacing: u8,
    pub padding: u8,
}
```

### 字段说明

- `regions` - 所有显示区域
- `font_size` - 字体大小（像素）
- `line_spacing` - 行间距（像素）
- `padding` - 内边距（像素）

### 使用示例

```rust
let layout = DisplayLayout {
    regions: vec![
        DisplayRegion { x: 0, y: 0, width: 800, height: 96, name: "time_region".to_string() },
        DisplayRegion { x: 0, y: 96, width: 800, height: 144, name: "weather_region".to_string() },
        DisplayRegion { x: 0, y: 240, width: 800, height: 240, name: "quote_region".to_string() },
    ],
    font_size: 24,
    line_spacing: 4,
    padding: 8,
};
```

## 数据类型定义

### ColorMode

```rust
pub enum ColorMode {
    Monochrome,
    Color3,
    Color4,
}
```

### RefreshError

```rust
pub enum RefreshError {
    Timeout,
    CommunicationError,
    HardwareError,
}
```

### DisplayColor

```rust
pub enum DisplayColor {
    Black,
    White,
}
```

## 实现注意事项

### 显示缓冲区管理

- 使用heapless::Vec避免动态内存分配（ESP32-C6）
- 缓冲区大小：800×480像素 = 480KB（黑白模式）
- 支持局部刷新，减少数据传输量
- 刷新期间主CPU进入轻睡眠

### 显示区域管理

- 预定义显示区域：时间区域、天气区域、格言区域
- 支持动态更新指定区域
- 避免全屏刷新，降低功耗

### 刷新状态管理

- 非阻塞刷新状态机
- 刷新期间不接受新的刷新请求
- BUSY信号超时：10秒
- 刷新失败后尝试复位屏幕

### 低电量优化

- 低电量时仅刷新核心时间信息
- 关闭天气和格言区域刷新
- 减少刷新频率，降低功耗

## 性能要求

- 缓冲区分配延迟：≤10ms
- 区域更新延迟：≤100ms
- 全屏刷新时间：≤10秒（含BUSY等待）
- 局部刷新时间：≤5秒（含BUSY等待）
- 状态切换延迟：≤1ms
