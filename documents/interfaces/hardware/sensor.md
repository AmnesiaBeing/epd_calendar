# Sensor (传感器) 接口文档

## 接口概述

Sensor接口负责管理温湿度传感器。本项目使用SHT20温湿度传感器，通过I2C接口读取数据。

## TemperatureSensor Trait

### 接口定义

```rust
pub trait TemperatureSensor {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn read_temperature(&self) -> Result<f32, Self::Error>;
    async fn read_humidity(&self) -> Result<f32, Self::Error>;
    async fn read_both(&self) -> Result<(f32, f32), Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化传感器，配置I2C接口
- `read_temperature()` - 读取温度（摄氏度）
- `read_humidity()` - 读取湿度（百分比）
- `read_both()` - 同时读取温度和湿度

### 关联类型

- `Error` - 传感器操作错误类型

### 使用示例

```rust
let mut sensor = Sht20::new(i2c);
sensor.initialize().await?;
let (temp, humidity) = sensor.read_both().await?;
lxxcc::info!("Temperature: {}°C, Humidity: {}%", temp, humidity);
```

## HumiditySensor Trait

### 接口定义

```rust
pub trait HumiditySensor {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn read_humidity(&self) -> Result<f32, Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化传感器，配置I2C接口
- `read_humidity()` - 读取湿度（百分比）

### 关联类型

- `Error` - 传感器操作错误类型

### 使用示例

```rust
let mut sensor = Sht20::new(i2c);
sensor.initialize().await?;
let humidity = sensor.read_humidity().await?;
lxxcc::info!("Humidity: {}%", humidity);
```

## 数据类型定义

### SensorData

```rust
pub struct SensorData {
    pub temperature: f32,
    pub humidity: f32,
    pub timestamp: u64,
}
```

## 实现注意事项

### ESP32-C6平台

- 使用LP I2C接口读取传感器
- 传感器读取由LPU负责，每分钟一次
- 数据写入RTC共享内存
- 读取失败时使用上次有效数据

### 泰山派平台

- 使用标准I2C接口读取传感器
- 传感器读取由主CPU负责，每分钟一次
- 数据写入NVS存储
- 读取失败时使用上次有效数据

### 模拟器平台

- 使用虚拟传感器，返回随机数据
- 传感器读取由主CPU负责，每分钟一次
- 数据写入内存
- 读取失败时返回默认值

## 性能要求

- 初始化时间：≤100ms
- 温度读取延迟：≤100ms
- 湿度读取延迟：≤100ms
- 同时读取延迟：≤100ms
- 温度精度：±0.5°C
- 湿度精度：±5%
