# RTC (实时时钟) 接口文档

## 接口概述

RTC (Real-Time Clock) 是实时时钟接口，负责管理系统时间、闹钟和时区。RTC在深度睡眠时保持运行，确保时间准确性。

## RTCTime Trait

### 接口定义

```rust
pub trait RTCTime {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn get_time(&self) -> Result<DateTime, Self::Error>;
    async fn set_time(&mut self, datetime: DateTime) -> Result<(), Self::Error>;
    async fn get_timezone(&self) -> Result<i32, Self::Error>;
    async fn set_timezone(&mut self, offset_seconds: i32) -> Result<(), Self::Error>;
    async fn calibrate(&mut self, offset_ms: i32) -> Result<(), Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化RTC，配置时钟源和时区
- `get_time()` - 获取当前时间
- `set_time()` - 设置当前时间
- `get_timezone()` - 获取时区偏移（相对于UTC的秒数）
- `set_timezone()` - 设置时区偏移
- `calibrate()` - 校准RTC，设置时间偏移（毫秒）

### 关联类型

- `Error` - RTC操作错误类型

### 使用示例

```rust
let mut rtc = Esp32C6Rtc::new(peripherals.rtc);
rtc.initialize().await?;
let now = rtc.get_time().await?;
rtc.set_timezone(28800).await?; // UTC+8
```

## RTCAlarm Trait

### 接口定义

```rust
pub trait RTCAlarm {
    type Error;

    async fn set_alarm(&mut self, alarm: Alarm) -> Result<(), Self::Error>;
    async fn get_alarm(&self, index: usize) -> Result<Alarm, Self::Error>;
    async fn clear_alarm(&mut self, index: usize) -> Result<(), Self::Error>;
    async fn enable_alarm(&mut self, index: usize) -> Result<(), Self::Error>;
    async fn disable_alarm(&mut self, index: usize) -> Result<(), Self::Error>;
    async fn get_triggered_alarms(&self) -> Result<Vec<usize>, Self::Error>;
}
```

### 方法说明

- `set_alarm()` - 设置闹钟（最多3个）
- `get_alarm()` - 获取指定闹钟配置
- `clear_alarm()` - 清除闹钟
- `enable_alarm()` - 启用闹钟
- `disable_alarm()` - 禁用闹钟
- `get_triggered_alarms()` - 获取已触发的闹钟列表

### 关联类型

- `Error` - 闹钟操作错误类型

### 使用示例

```rust
let alarm = Alarm {
    hour: 7,
    minute: 30,
    repeat: Repeat::Daily,
    enabled: true,
};
rtc.set_alarm(alarm).await?;
```

## 数据类型定义

### DateTime

```rust
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}
```

### Alarm

```rust
pub struct Alarm {
    pub hour: u8,
    pub minute: u8,
    pub repeat: Repeat,
    pub enabled: bool,
}
```

### Repeat

```rust
pub enum Repeat {
    Once,
    Daily,
    Weekday(u8), // 0-6, 0=Monday
    Weekend,
}
```

## 实现注意事项

### ESP32-C6平台

- 使用ESP32-C6内置RTC
- RTC在深度睡眠时保持运行
- 闹钟通过RTC中断唤醒主CPU
- 时区偏移存储在RTC寄存器中

### 泰山派平台

- 使用Linux系统RTC
- RTC在深度睡眠时保持运行
- 闹钟通过RTC中断唤醒主CPU
- 时区偏移存储在NVS中

### 模拟器平台

- 使用系统时间模拟RTC
- 闹钟通过定时器模拟
- 时区偏移存储在内存中

## 性能要求

- 初始化时间：≤100ms
- 时间读取延迟：≤10ms
- 时间设置延迟：≤10ms
- 时区设置延迟：≤10ms
- 校准延迟：≤10ms
- 闹钟设置延迟：≤10ms
- 闹钟触发精度：±1秒
