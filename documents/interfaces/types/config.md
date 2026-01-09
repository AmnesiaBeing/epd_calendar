# 配置 (Config) 数据类型文档

## 概述

配置类型用于管理系统配置，包括时间配置、网络配置、显示配置和系统配置。配置数据存储在NVS中，敏感数据加密存储。

## SystemConfig

### 结构定义

```rust
pub struct SystemConfig {
    pub version: u32,
    pub time_config: TimeConfig,
    pub network_config: NetworkConfig,
    pub display_config: DisplayConfig,
    pub power_config: PowerConfig,
    pub log_config: LogConfig,
}
```

### 字段说明

- `version` - 配置版本号
- `time_config` - 时间配置
- `network_config` - 网络配置
- `display_config` - 显示配置
- `power_config` - 电源配置
- `log_config` - 日志配置

### 使用示例

```rust
let config = SystemConfig {
    version: 1,
    time_config: TimeConfig { ... },
    network_config: NetworkConfig { ... },
    display_config: DisplayConfig { ... },
    power_config: PowerConfig { ... },
    log_config: LogConfig { ... },
};
```

## TimeConfig

### 结构定义

```rust
pub struct TimeConfig {
    pub timezone_offset: i32,
    pub alarms: Vec<AlarmConfig>,
    pub hour_chime_enabled: bool,
    pub auto_sleep_start: Option<TimeOfDay>,
    pub auto_sleep_end: Option<TimeOfDay>,
}
```

### 字段说明

- `timezone_offset` - 时区偏移（相对于UTC的秒数）
- `alarms` - 闹钟列表（最多3个）
- `hour_chime_enabled` - 整点报时开关
- `auto_sleep_start` - 自动休眠开始时间
- `auto_sleep_end` - 自动休眠结束时间

### 使用示例

```rust
let time_config = TimeConfig {
    timezone_offset: 28800,
    alarms: vec![
        AlarmConfig { hour: 7, minute: 30, repeat: Repeat::Daily, enabled: true },
    ],
    hour_chime_enabled: true,
    auto_sleep_start: Some(TimeOfDay { hour: 23, minute: 0 }),
    auto_sleep_end: Some(TimeOfDay { hour: 7, minute: 0 }),
};
```

## NetworkConfig

### 结构定义

```rust
pub struct NetworkConfig {
    pub wifi_ssid: String,
    pub wifi_password: EncryptedString,
    pub weather_api_key: EncryptedString,
    pub location_id: String,
    pub sync_interval_minutes: u16,
}
```

### 字段说明

- `wifi_ssid` - Wi-Fi SSID
- `wifi_password` - Wi-Fi密码（加密）
- `weather_api_key` - 和风天气API密钥（加密）
- `location_id` - 位置ID（城市ID）
- `sync_interval_minutes` - 同步间隔（分钟）

### 使用示例

```rust
let network_config = NetworkConfig {
    wifi_ssid: "MyWiFi".to_string(),
    wifi_password: EncryptedString { data: vec![...], iv: vec![...] },
    weather_api_key: EncryptedString { data: vec![...], iv: vec![...] },
    location_id: "101010100".to_string(),
    sync_interval_minutes: 120,
};
```

## DisplayConfig

### 结构定义

```rust
pub struct DisplayConfig {
    pub theme: DisplayTheme,
    pub low_power_refresh_enabled: bool,
    pub refresh_interval_seconds: u16,
}
```

### 字段说明

- `theme` - 显示主题
- `low_power_refresh_enabled` - 低电量刷新开关
- `refresh_interval_seconds` - 刷新间隔（秒）

### 使用示例

```rust
let display_config = DisplayConfig {
    theme: DisplayTheme::Default,
    low_power_refresh_enabled: true,
    refresh_interval_seconds: 60,
};
```

## PowerConfig

### 结构定义

```rust
pub struct PowerConfig {
    pub low_battery_threshold: u8,
    pub critical_battery_threshold: u8,
    pub low_power_mode_enabled: bool,
}
```

### 字段说明

- `low_battery_threshold` - 低电量阈值（0-100%）
- `critical_battery_threshold` - 严重低电量阈值（0-100%）
- `low_power_mode_enabled` - 低电量模式开关

### 使用示例

```rust
let power_config = PowerConfig {
    low_battery_threshold: 30,
    critical_battery_threshold: 10,
    low_power_mode_enabled: true,
};
```

## LogConfig

### 结构定义

```rust
pub struct LogConfig {
    pub log_mode: LogMode,
    pub log_level: LogLevel,
    pub log_to_flash: bool,
}
```

### 字段说明

- `log_mode` - 日志模式
- `log_level` - 日志级别
- `log_to_flash` - 是否记录到FLASH

### 使用示例

```rust
let log_config = LogConfig {
    log_mode: LogMode::Defmt,
    log_level: LogLevel::Info,
    log_to_flash: true,
};
```

## 数据类型定义

### AlarmConfig

```rust
pub struct AlarmConfig {
    pub hour: u8,
    pub minute: u8,
    pub repeat: Repeat,
    pub enabled: bool,
}
```

### TimeOfDay

```rust
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
}
```

### EncryptedString

```rust
pub struct EncryptedString {
    pub data: Vec<u8>,
    pub iv: Vec<u8>,
}
```

### DisplayTheme

```rust
pub enum DisplayTheme {
    Default,
}
```

### LogMode

```rust
pub enum LogMode {
    Log,
    Defmt,
    None,
}
```

### LogLevel

```rust
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}
```

## 实现注意事项

### 配置存储

- 配置数据存储在NVS中
- 敏感数据加密存储（ESP32-C6使用RSA硬件加密）
- 配置变更时标记脏标志
- 定期保存到NVS，避免频繁擦写

### 配置加载

- 从NVS加载配置到内存
- 配置版本管理和兼容性处理
- 加载失败时使用默认配置
- 支持旧配置平滑迁移

### 配置加密

- ESP32-C6：使用RSA硬件加密
- 泰山派：使用软件加密
- 模拟器：不加密，标注安全风险
- 敏感数据：Wi-Fi密码、API密钥

### 配置验证

- 配置加载后验证数据完整性
- 检查配置版本兼容性
- 验证闹钟时间有效性
- 验证时区偏移有效性

## 性能要求

- 配置加载延迟：≤100ms
- 配置保存延迟：≤500ms
- 加密/解密延迟：≤100ms
- 配置验证延迟：≤10ms
