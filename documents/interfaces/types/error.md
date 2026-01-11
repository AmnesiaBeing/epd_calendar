# 错误 (Error) 数据类型文档

## 概述

错误类型用于定义系统错误，包括驱动错误、服务错误和系统错误。所有错误类型实现`core::fmt::Display`和`core::fmt::Debug`trait。

## SystemError

### 枚举定义

```rust
pub enum SystemError {
    DriverError(DriverError),
    ServiceError(ServiceError),
    ConfigError(ConfigError),
    HardwareError(HardwareError),
    NetworkError(NetworkError),
    StorageError(StorageError),
}
```

### 变体说明

- `DriverError` - 驱动错误
- `ServiceError` - 服务错误
- `ConfigError` - 配置错误
- `HardwareError` - 硬件错误
- `NetworkError` - 网络错误
- `StorageError` - 存储错误

### 使用示例

```rust
match result {
    Ok(data) => lxx_common::info!("Success: {:?}", data),
    Err(SystemError::DriverError(err)) => lxx_common::error!("Driver error: {}", err),
    Err(SystemError::ServiceError(err)) => lxx_common::error!("Service error: {}", err),
    Err(err) => lxx_common::error!("System error: {}", err),
}
```

## DriverError

### 枚举定义

```rust
pub enum DriverError {
    EPDError(EPDError),
    RTCError(RTCError),
    SensorError(SensorError),
    BuzzerError(BuzzerError),
    ButtonError(ButtonError),
    LEDError(LEDError),
    WiFiError(WiFiError),
    BLEError(BLEError),
    BatteryError(BatteryError),
}
```

### 变体说明

- `EPDError` - 墨水屏驱动错误
- `RTCError` - RTC驱动错误
- `SensorError` - 传感器驱动错误
- `BuzzerError` - 蜂鸣器驱动错误
- `ButtonError` - 按键驱动错误
- `LEDError` - LED驱动错误
- `WiFiError` - Wi-Fi驱动错误
- `BLEError` - BLE驱动错误
- `BatteryError` - 电池驱动错误

## ServiceError

### 枚举定义

```rust
pub enum ServiceError {
    StateManagerError(StateManagerError),
    TimeServiceError(TimeServiceError),
    NetworkServiceError(NetworkServiceError),
    DisplayServiceError(DisplayServiceError),
    BLEServiceError(BLEServiceError),
    AudioServiceError(AudioServiceError),
    ConfigManagerError(ConfigManagerError),
    PowerManagerError(PowerManagerError),
}
```

### 变体说明

- `StateManagerError` - 状态管理器错误
- `TimeServiceError` - 时间服务错误
- `NetworkServiceError` - 网络服务错误
- `DisplayServiceError` - 显示服务错误
- `BLEServiceError` - 蓝牙服务错误
- `AudioServiceError` - 音频服务错误
- `ConfigManagerError` - 配置管理器错误
- `PowerManagerError` - 电源管理器错误

## HardwareError

### 枚举定义

```rust
pub enum HardwareError {
    Timeout,
    CommunicationError,
    InvalidParameter,
    HardwareFault,
    NotInitialized,
    AlreadyInitialized,
}
```

### 变体说明

- `Timeout` - 操作超时
- `CommunicationError` - 通信错误
- `InvalidParameter` - 无效参数
- `HardwareFault` - 硬件故障
- `NotInitialized` - 未初始化
- `AlreadyInitialized` - 已初始化

## NetworkError

### 枚举定义

```rust
pub enum NetworkError {
    ConnectionFailed,
    AuthenticationFailed,
    DNSResolutionFailed,
    Timeout,
    NoRouteToHost,
    ConnectionRefused,
    HTTPError(u16),
    ParseError,
}
```

### 变体说明

- `ConnectionFailed` - 连接失败
- `AuthenticationFailed` - 认证失败
- `DNSResolutionFailed` - DNS解析失败
- `Timeout` - 超时
- `NoRouteToHost` - 无路由到主机
- `ConnectionRefused` - 连接被拒绝
- `HTTPError` - HTTP错误（状态码）
- `ParseError` - 解析错误

## StorageError

### 枚举定义

```rust
pub enum StorageError {
    ReadFailed,
    WriteFailed,
    EraseFailed,
    NotEnoughSpace,
    CorruptedData,
    NotFound,
    EncryptionError,
}
```

### 变体说明

- `ReadFailed` - 读取失败
- `WriteFailed` - 写入失败
- `EraseFailed` - 擦除失败
- `NotEnoughSpace` - 空间不足
- `CorruptedData` - 数据损坏
- `NotFound` - 未找到
- `EncryptionError` - 加密错误

## 数据类型定义

### EPDError

```rust
pub enum EPDError {
    Timeout,
    CommunicationError,
    BusyTimeout,
    InitializationFailed,
}
```

### RTCError

```rust
pub enum RTCError {
    ReadFailed,
    WriteFailed,
    InvalidTime,
    AlarmFailed,
}
```

### SensorError

```rust
pub enum SensorError {
    ReadFailed,
    Timeout,
    InvalidData,
    NotConnected,
}
```

### BuzzerError

```rust
pub enum BuzzerError {
    NotInitialized,
    InvalidFrequency,
    Timeout,
}
```

### ButtonError

```rust
pub enum ButtonError {
    Timeout,
    DebounceFailed,
    NotInitialized,
}
```

### LEDError

```rust
pub enum LEDError {
    NotInitialized,
    InvalidBrightness,
    Timeout,
}
```

### WiFiError

```rust
pub enum WiFiError {
    ConnectionFailed,
    AuthenticationFailed,
    Timeout,
    ScanFailed,
}
```

### BLEError

```rust
pub enum BLEError {
    AdvertisingFailed,
    ConnectionFailed,
    NotInitialized,
    Timeout,
}
```

### BatteryError

```rust
pub enum BatteryError {
    ReadFailed,
    CalibrationFailed,
    Timeout,
}
```

## 实现注意事项

### 错误处理

- 所有错误类型实现`Display`和`Debug`trait
- 错误消息使用中文，便于调试
- 错误代码使用枚举，便于匹配
- 错误链：使用`source()`方法追踪错误来源

### 错误恢复

- 可恢复错误：记录日志，使用默认值
- 需用户干预错误：显示错误提示，等待用户操作
- 系统保护错误：触发系统保护机制

### 错误日志

- 所有错误记录到日志
- 核心错误记录到FLASH固定分区
- 敏感数据脱敏，避免泄露
- 错误堆栈记录（ESP32-C6使用`esp-backtrace`）

### 错误传播

- 使用`?`操作符简化错误传播
- 使用`map_err()`转换错误类型
- 使用`context()`添加错误上下文

## 性能要求

- 错误创建延迟：≤1ms
- 错误传播延迟：≤1ms
- 错误日志延迟：≤10ms
- 错误恢复延迟：≤100ms
