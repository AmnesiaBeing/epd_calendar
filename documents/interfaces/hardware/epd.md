# EPD (电子墨水屏) 接口文档

## 接口概述

EPD (Electronic Paper Display) 是电子墨水屏驱动接口，负责控制墨水屏的显示和刷新。本项目使用YRD0750RYF665F60型号，7.5英寸，800×480分辨率，黑白红黄4色（实际仅使用黑白2色）。

## EPDDriver Trait

### 接口定义

```rust
pub trait EPDDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn clear(&mut self, color: DisplayColor) -> Result<(), Self::Error>;
    async fn update(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;
    async fn update_partial(&mut self, x: u16, y: u16, width: u16, height: u16, buffer: &[u8]) -> Result<(), Self::Error>;
    async fn sleep(&mut self) -> Result<(), Self::Error>;
    async fn wakeup(&mut self) -> Result<(), Self::Error>;
    async fn get_refresh_state(&self) -> Result<RefreshState, Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化墨水屏，配置SPI接口和显示参数
- `clear()` - 清屏，使用指定颜色（黑色/白色）
- `update()` - 全屏刷新，发送完整显示缓冲区数据
- `update_partial()` - 局部刷新，更新指定区域的显示内容
- `sleep()` - 进入睡眠模式，关闭显示
- `wakeup()` - 唤醒显示，从睡眠模式恢复
- `get_refresh_state()` - 获取当前刷新状态

### 关联类型

- `Error` - EPD操作错误类型

### 使用示例

```rust
let mut epd = Yrd0750ryf665f60::new(spi, cs, dc, rst, busy);
epd.initialize().await?;
epd.clear(DisplayColor::White).await?;
epd.update(&display_buffer).await?;
```

## EPDRefresh Trait

### 接口定义

```rust
pub trait EPDRefresh {
    type Error;

    async fn start_refresh(&mut self) -> Result<(), Self::Error>;
    async fn wait_busy(&mut self, timeout: Duration) -> Result<bool, Self::Error>;
    async fn cancel_refresh(&mut self) -> Result<(), Self::Error>;
    async fn get_refresh_progress(&self) -> Result<u8, Self::Error>;
}
```

### 方法说明

- `start_refresh()` - 开始刷新流程
- `wait_busy()` - 等待BUSY信号，超时返回false
- `cancel_refresh()` - 取消当前刷新
- `get_refresh_progress()` - 获取刷新进度（0-100%）

### 关联类型

- `Error` - 刷新操作错误类型

### 使用示例

```rust
epd.start_refresh().await?;
let success = epd.wait_busy(Duration::from_secs(10)).await?;
if !success {
    epd.cancel_refresh().await?;
}
```

## EPDRegion Trait

### 接口定义

```rust
pub trait EPDRegion {
    type Error;

    async fn set_region(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<(), Self::Error>;
    async fn get_region(&self) -> Result<Region, Self::Error>;
    async fn clear_region(&mut self, region: Region) -> Result<(), Self::Error>;
}
```

### 方法说明

- `set_region()` - 设置当前操作区域
- `get_region()` - 获取当前操作区域
- `clear_region()` - 清除指定区域

### 关联类型

- `Error` - 区域操作错误类型

### 使用示例

```rust
epd.set_region(0, 0, 400, 240).await?;
epd.clear_region(Region { x: 0, y: 0, width: 400, height: 240 }).await?;
```

## 数据类型定义

### DisplayColor

```rust
pub enum DisplayColor {
    Black,
    White,
}
```

### RefreshState

```rust
pub enum RefreshState {
    Idle,
    SendingData,
    Refreshing,
    Error,
}
```

### Region

```rust
pub struct Region {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
```

## 实现注意事项

### ESP32-C6平台

- 使用SPI接口驱动墨水屏
- BUSY信号通过GPIO中断检测
- 刷新期间主CPU进入轻睡眠
- 非阻塞刷新状态机，避免主CPU空闲等待

### 泰山派平台

- 使用SPI接口驱动墨水屏
- BUSY信号通过轮询检测
- 刷新期间主CPU进入轻睡眠

### 模拟器平台

- 使用SDL2窗口模拟墨水屏
- 无BUSY信号，使用定时器模拟刷新延迟
- 刷新期间主CPU进入轻睡眠

## 性能要求

- 初始化时间：≤500ms
- 清屏时间：≤500ms
- 全屏刷新时间：≤10秒（含BUSY等待）
- 局部刷新时间：≤5秒（含BUSY等待）
- BUSY检测延迟：≤10ms
