# LED (LED指示灯) 接口文档

## 接口概述

LED接口负责控制LED指示灯，用于显示设备状态（配网状态、低电量等）。本项目使用低电平点亮的LED。

## LEDDriver Trait

### 接口定义

```rust
pub trait LEDDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn on(&mut self) -> Result<(), Self::Error>;
    async fn off(&mut self) -> Result<(), Self::Error>;
    async fn toggle(&mut self) -> Result<(), Self::Error>;
    async fn set_brightness(&mut self, brightness: u8) -> Result<(), Self::Error>;
    async fn blink(&mut self, interval: Duration, count: u32) -> Result<(), Self::Error>;
    async fn is_on(&self) -> Result<bool, Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化LED，配置GPIO
- `on()` - 打开LED（低电平）
- `off()` - 关闭LED（高电平）
- `toggle()` - 切换LED状态
- `set_brightness()` - 设置LED亮度（0-100%）
- `blink()` - 闪烁LED，指定间隔和次数
- `is_on()` - 检测LED是否打开

### 关联类型

- `Error` - LED操作错误类型

### 使用示例

```rust
let mut led = Esp32C6Led::new(gpio_pin);
led.initialize().await?;
led.on().await?;
led.blink(Duration::from_millis(500), 3).await?;
```

## LEDControl Trait

### 接口定义

```rust
pub trait LEDControl {
    type Error;

    async fn set_state(&mut self, state: LEDState) -> Result<(), Self::Error>;
    async fn get_state(&self) -> Result<LEDState, Self::Error>;
    async fn start_blinking(&mut self, interval: Duration) -> Result<(), Self::Error>;
    async fn stop_blinking(&mut self) -> Result<(), Self::Error>;
}
```

### 方法说明

- `set_state()` - 设置LED状态（开/关/闪烁）
- `get_state()` - 获取LED状态
- `start_blinking()` - 开始闪烁
- `stop_blinking()` - 停止闪烁

### 关联类型

- `Error` - LED控制错误类型

### 使用示例

```rust
led.set_state(LEDState::Blinking(Duration::from_millis(500))).await?;
```

## 数据类型定义

### LEDState

```rust
pub enum LEDState {
    On,
    Off,
    Blinking(Duration),
}
```

## 实现注意事项

### ESP32-C6平台

- 使用GPIO控制LED
- 低电平点亮，高电平熄灭
- 未配网时快闪（2Hz）
- 已配网时慢闪（0.5Hz）
- 低电量时减少闪烁频率

### 泰山派平台

- 使用GPIO控制LED
- 低电平点亮，高电平熄灭
- 未配网时快闪（2Hz）
- 已配网时慢闪（0.5Hz）
- 低电量时减少闪烁频率

### 模拟器平台

- 使用窗口标题栏图标模拟LED
- 未配网时快闪（2Hz）
- 已配网时慢闪（0.5Hz）
- 低电量时减少闪烁频率

## 性能要求

- 初始化时间：≤10ms
- 打开/关闭延迟：≤1ms
- 亮度设置延迟：≤1ms
- 闪烁精度：±10ms
- 状态检测延迟：≤1ms
