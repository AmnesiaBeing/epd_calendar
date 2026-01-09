# Button (按键) 接口文档

## 接口概述

Button接口负责管理按键输入，支持短按和长按检测。按键用于唤醒主CPU、进入蓝牙配网模式、恢复出厂设置等操作。

## ButtonDriver Trait

### 接口定义

```rust
pub trait ButtonDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn is_pressed(&self) -> Result<bool, Self::Error>;
    async fn wait_for_press(&mut self, timeout: Duration) -> Result<ButtonEvent, Self::Error>;
    async fn wait_for_release(&mut self, timeout: Duration) -> Result<(), Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化按键，配置GPIO和中断
- `is_pressed()` - 检测按键是否按下
- `wait_for_press()` - 等待按键按下，返回按键事件
- `wait_for_release()` - 等待按键释放

### 关联类型

- `Error` - 按键操作错误类型

### 使用示例

```rust
let mut button = Esp32C6Button::new(gpio_pin);
button.initialize().await?;
let event = button.wait_for_press(Duration::from_secs(30)).await?;
match event {
    ButtonEvent::ShortPress => lxxcc::info!("Short press"),
    ButtonEvent::LongPress => lxxcc::info!("Long press"),
}
```

## ButtonEvent Trait

### 接口定义

```rust
pub trait ButtonEvent {
    fn is_short_press(&self) -> bool;
    fn is_long_press(&self) -> bool;
    fn get_duration(&self) -> Duration;
}
```

### 方法说明

- `is_short_press()` - 判断是否为短按（<3秒）
- `is_long_press()` - 判断是否为长按（≥15秒）
- `get_duration()` - 获取按键持续时间

### 使用示例

```rust
if event.is_short_press() {
    lxxcc::info!("Short press detected");
} else if event.is_long_press() {
    lxxcc::info!("Long press detected");
}
```

## 数据类型定义

### ButtonEvent

```rust
pub enum ButtonEvent {
    ShortPress,
    LongPress,
}
```

### ButtonState

```rust
pub enum ButtonState {
    Released,
    Pressed,
}
```

## 实现注意事项

### ESP32-C6平台

- 按键由LPU（LP Core）负责监控
- 支持外部上拉，按下检测
- 短按检测：<3秒
- 长按检测：≥15秒
- 非必要时不唤醒主CPU
- 通过GPIO中断唤醒主CPU

### 泰山派平台

- 按键由主CPU负责监控
- 支持外部上拉，按下检测
- 短按检测：<3秒
- 长按检测：≥15秒
- 通过GPIO中断检测按键

### 模拟器平台

- 按键由主CPU负责监控
- 使用键盘按键模拟
- 短按检测：<3秒
- 长按检测：≥15秒
- 通过键盘事件检测按键

## 性能要求

- 初始化时间：≤10ms
- 按键检测延迟：≤100ms（去抖后）
- 短按判断精度：±100ms
- 长按判断精度：±100ms
- 唤醒延迟：≤50ms
