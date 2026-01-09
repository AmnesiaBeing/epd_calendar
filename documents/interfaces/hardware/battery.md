# Battery (电池) 接口文档

## 接口概述

Battery接口负责检测电池电量，用于低电量保护。本项目使用ADC检测电池电压，按2档电量显示（正常/低电量）。

## BatteryDriver Trait

### 接口定义

```rust
pub trait BatteryDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn read_voltage(&self) -> Result<f32, Self::Error>;
    async fn read_percentage(&self) -> Result<u8, Self::Error>;
    async fn is_low_battery(&self) -> Result<bool, Self::Error>;
    async fn is_charging(&self) -> Result<bool, Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化电池检测，配置ADC
- `read_voltage()` - 读取电池电压（V）
- `read_percentage()` - 读取电池电量百分比（0-100%）
- `is_low_battery()` - 检测是否低电量（<30%）
- `is_charging()` - 检测是否正在充电

### 关联类型

- `Error` - 电池操作错误类型

### 使用示例

```rust
let mut battery = Esp32C6Battery::new(adc_pin);
battery.initialize().await?;
let voltage = battery.read_voltage().await?;
let percentage = battery.read_percentage().await?;
lxxcc::info!("Voltage: {}V, Percentage: {}%", voltage, percentage);
if battery.is_low_battery().await? {
    lxxcc::warn!("Low battery!");
}
```

## BatteryMonitor Trait

### 接口定义

```rust
pub trait BatteryMonitor {
    type Error;

    async fn calibrate(&mut self, full_voltage: f32, empty_voltage: f32) -> Result<(), Self::Error>;
    async fn get_calibration(&self) -> Result<BatteryCalibration, Self::Error>;
    async fn set_low_battery_threshold(&mut self, threshold: u8) -> Result<(), Self::Error>;
    async fn get_low_battery_threshold(&self) -> Result<u8, Self::Error>;
}
```

### 方法说明

- `calibrate()` - 校准电池，设置满电和空电电压
- `get_calibration()` - 获取校准数据
- `set_low_battery_threshold()` - 设置低电量阈值（默认30%）
- `get_low_battery_threshold()` - 获取低电量阈值

### 关联类型

- `Error` - 电池监控错误类型

### 使用示例

```rust
battery.calibrate(4.2, 3.0).await?; // 满电4.2V，空电3.0V
battery.set_low_battery_threshold(30).await?;
```

## 数据类型定义

### BatteryCalibration

```rust
pub struct BatteryCalibration {
    pub full_voltage: f32,
    pub empty_voltage: f32,
    pub timestamp: u64,
}
```

### BatteryLevel

```rust
pub enum BatteryLevel {
    Normal,
    Low,
    Critical,
}
```

## 实现注意事项

### ESP32-C6平台

- 使用ADC检测电池电压
- 按2档电量显示（正常/低电量）
- 正常电量：≥30%
- 低电量：<30%
- 低电量保护：<10%，关闭所有非必要功能
- 电量校准：首次上电及每充电一次
- 校准数据存储在NVS中

### 泰山派平台

- 使用ADC检测电池电压
- 按2档电量显示（正常/低电量）
- 正常电量：≥30%
- 低电量：<30%
- 低电量保护：<10%，关闭所有非必要功能
- 电量校准：首次上电及每充电一次
- 校准数据存储在NVS中

### 模拟器平台

- 使用虚拟电池，通过配置文件设置
- 按2档电量显示（正常/低电量）
- 正常电量：≥30%
- 低电量：<30%
- 低电量保护：<10%，关闭所有非必要功能
- 电量校准：通过配置文件设置
- 校准数据存储在配置文件中

## 性能要求

- 初始化时间：≤10ms
- 电压读取延迟：≤10ms
- 电量读取延迟：≤10ms
- 低电量检测延迟：≤10ms
- 充电检测延迟：≤10ms
- 校准延迟：≤10ms
- 电压精度：±0.05V
- 电量精度：±5%
