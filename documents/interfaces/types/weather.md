# 天气 (Weather) 数据类型文档

## 概述

天气类型用于管理天气信息，包括当前天气和天气预报。天气数据从和风天气API获取，缓存到NVS存储。

## WeatherInfo

### 结构定义

```rust
pub struct WeatherInfo {
    pub location: String,
    pub current: CurrentWeather,
    pub forecast: Vec<ForecastWeather>,
    pub last_update: u64,
}
```

### 字段说明

- `location` - 位置名称（例如："上海"）
- `current` - 当前天气
- `forecast` - 天气预报（3天）
- `last_update` - 上次更新时间（Unix时间戳，秒）

### 使用示例

```rust
let weather = WeatherInfo {
    location: "上海".to_string(),
    current: CurrentWeather { ... },
    forecast: vec![ ... ],
    last_update: 1705315200,
};
```

## CurrentWeather

### 结构定义

```rust
pub struct CurrentWeather {
    pub temp: i16,
    pub feels_like: i16,
    pub humidity: u8,
    pub condition: WeatherCondition,
    pub wind_speed: u8,
    pub wind_direction: u16,
    pub visibility: u16,
    pub pressure: u16,
    pub update_time: u64,
}
```

### 字段说明

- `temp` - 当前温度（0.1°C单位，例如：220表示22.0°C）
- `feels_like` - 体感温度（0.1°C单位）
- `humidity` - 相对湿度（0-100%）
- `condition` - 天气状况
- `wind_speed` - 风速（km/h）
- `wind_direction` - 风向（度，0-360）
- `visibility` - 能见度（km）
- `pressure` - 气压（hPa）
- `update_time` - 更新时间（Unix时间戳，秒）

### 使用示例

```rust
let current = CurrentWeather {
    temp: 220,
    feels_like: 218,
    humidity: 65,
    condition: WeatherCondition::Cloudy,
    wind_speed: 10,
    wind_direction: 180,
    visibility: 10,
    pressure: 1013,
    update_time: 1705315200,
};
```

## ForecastWeather

### 结构定义

```rust
pub struct ForecastWeather {
    pub date: DateTime,
    pub temp_max: i16,
    pub temp_min: i16,
    pub condition: WeatherCondition,
    pub humidity: u8,
    pub wind_speed: u8,
}
```

### 字段说明

- `date` - 预报日期
- `temp_max` - 最高温度（0.1°C单位）
- `temp_min` - 最低温度（0.1°C单位）
- `condition` - 天气状况
- `humidity` - 相对湿度（0-100%）
- `wind_speed` - 风速（km/h）

### 使用示例

```rust
let forecast = ForecastWeather {
    date: DateTime { ... },
    temp_max: 250,
    temp_min: 180,
    condition: WeatherCondition::Sunny,
    humidity: 60,
    wind_speed: 12,
};
```

## 数据类型定义

### WeatherCondition

```rust
pub enum WeatherCondition {
    Sunny,
    Cloudy,
    Overcast,
    Rain,
    Snow,
    Thunderstorm,
    Fog,
    Haze,
}
```

### WeatherCode

```rust
pub struct WeatherCode {
    pub code: String,
    pub description: String,
    pub icon: WeatherIcon,
}
```

### WeatherIcon

```rust
pub enum WeatherIcon {
    Sunny,
    Cloudy,
    Overcast,
    Rain,
    Snow,
    Thunderstorm,
    Fog,
    Haze,
}
```

## 实现注意事项

### 天气API

- 使用和风天气API获取天气数据
- API密钥加密存储（ESP32-C6使用RSA硬件加密）
- 位置ID配置到NVS存储
- API调用限制管理（月≤20,000次）

### 天气缓存

- 天气数据缓存到NVS存储
- 每2小时更新一次（正常电量）
- 低电量时延长至4小时
- 缓存有效期：4小时

### 天气显示

- 显示当前天气：温度、湿度、天气状况
- 显示天气预报：3天预报
- 天气图标：使用embedded-graphics绘制
- 低电量时关闭天气区域

### 错误处理

- API调用失败：使用缓存数据
- 网络不可用：显示离线状态
- 数据解析失败：使用默认值

## 性能要求

- API调用延迟：≤5秒
- 数据解析延迟：≤100ms
- 缓存读写延迟：≤10ms
- 图标绘制延迟：≤100ms
- 温度精度：±0.5°C
- 湿度精度：±5%
