# Buzzer (蜂鸣器) 接口文档

## 接口概述

Buzzer接口负责控制蜂鸣器，用于整点报时和闹钟音乐播放。在ESP32-C6平台上，使用LEDC PWM驱动蜂鸣器。

## BuzzerDriver Trait

### 接口定义

```rust
pub trait BuzzerDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn set_frequency(&mut self, frequency: u32) -> Result<(), Self::Error>;
    async fn set_duty_cycle(&mut self, duty: u8) -> Result<(), Self::Error>;
    async fn beep(&mut self, duration: Duration) -> Result<(), Self::Error>;
    async fn play_tone(&mut self, frequency: u32, duration: Duration) -> Result<(), Self::Error>;
    async fn stop(&mut self) -> Result<(), Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化蜂鸣器，配置PWM接口
- `set_frequency()` - 设置蜂鸣器频率（Hz）
- `set_duty_cycle()` - 设置PWM占空比（0-100%）
- `beep()` - 播放默认频率的蜂鸣声
- `play_tone()` - 播放指定频率和持续时间的音调
- `stop()` - 停止蜂鸣器

### 关联类型

- `Error` - 蜂鸣器操作错误类型

### 使用示例

```rust
let mut buzzer = Esp32C6Buzzer::new(ledc, pin);
buzzer.initialize().await?;
buzzer.play_tone(440, Duration::from_millis(500)).await?; // A4音
buzzer.stop().await?;
```

## ToneGenerator Trait

### 接口定义

```rust
pub trait ToneGenerator {
    type Error;

    async fn play_melody(&mut self, melody: &[Tone]) -> Result<(), Self::Error>;
    async fn play_chime(&mut self) -> Result<(), Self::Error>;
    async fn set_volume(&mut self, volume: u8) -> Result<(), Self::Error>;
}
```

### 方法说明

- `play_melody()` - 播放旋律（音调序列）
- `play_chime()` - 播放整点报时（4短1长）
- `set_volume()` - 设置音量（0-100%）

### 关联类型

- `Error` - 音调生成错误类型

### 使用示例

```rust
let melody = vec![
    Tone { frequency: 523, duration: Duration::from_millis(500) }, // C5
    Tone { frequency: 587, duration: Duration::from_millis(500) }, // D5
    Tone { frequency: 659, duration: Duration::from_millis(500) }, // E5
];
buzzer.play_melody(&melody).await?;
```

## 数据类型定义

### Tone

```rust
pub struct Tone {
    pub frequency: u32,
    pub duration: Duration,
}
```

## 实现注意事项

### ESP32-C6平台

- 使用LEDC外设产生PWM
- PWM频率范围：100Hz-20kHz
- 音量通过PWM占空比控制
- 整点报时：4短（400ms间隔）+1长（500ms）
- 闹钟音乐：小星星、兰花草等预设旋律
- 低电量模式下禁用非闹钟音频

### 泰山派平台

- 使用Linux PWM驱动
- PWM频率范围：100Hz-20kHz
- 音量通过PWM占空比控制
- 整点报时：4短（400ms间隔）+1长（500ms）
- 闹钟音乐：小星星、兰花草等预设旋律
- 低电量模式下禁用非闹钟音频

### 模拟器平台

- 使用声卡播放音频
- 音量通过系统音量控制
- 整点报时：4短（400ms间隔）+1长（500ms）
- 闹钟音乐：小星星、兰花草等预设旋律
- 低电量模式下禁用非闹钟音频

## 性能要求

- 初始化时间：≤10ms
- 频率设置延迟：≤1ms
- 占空比设置延迟：≤1ms
- 蜂鸣声播放延迟：≤10ms
- 音调播放延迟：≤10ms
- 停止延迟：≤1ms
- 频率精度：±1%
- 占空比精度：±1%
