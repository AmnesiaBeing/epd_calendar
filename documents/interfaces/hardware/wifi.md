# Wi-Fi (无线网络) 接口文档

## 接口概述

Wi-Fi接口负责管理Wi-Fi连接，用于网络同步（时间、天气）。Wi-Fi按需连接，完成同步后立即断开，降低功耗。

## WiFiDriver Trait

### 接口定义

```rust
pub trait WiFiDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn connect(&mut self, ssid: &str, password: &str) -> Result<(), Self::Error>;
    async fn disconnect(&mut self) -> Result<(), Self::Error>;
    async fn is_connected(&self) -> Result<bool, Self::Error>;
    async fn get_rssi(&self) -> Result<i8, Self::Error>;
    async fn scan(&mut self) -> Result<Vec<WiFiNetwork>, Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化Wi-Fi模块
- `connect()` - 连接到指定Wi-Fi网络
- `disconnect()` - 断开Wi-Fi连接
- `is_connected()` - 检测是否已连接
- `get_rssi()` - 获取信号强度（dBm）
- `scan()` - 扫描附近的Wi-Fi网络

### 关联类型

- `Error` - Wi-Fi操作错误类型

### 使用示例

```rust
let mut wifi = Esp32C6WiFi::new(radio);
wifi.initialize().await?;
wifi.connect("MyWiFi", "password").await?;
if wifi.is_connected().await? {
    let rssi = wifi.get_rssi().await?;
    lxxcc::info!("RSSI: {} dBm", rssi);
}
```

## WiFiConnection Trait

### 接口定义

```rust
pub trait WiFiConnection {
    type Error;

    async fn get_ip_address(&self) -> Result<IpAddress, Self::Error>;
    async fn get_gateway(&self) -> Result<IpAddress, Self::Error>;
    async fn get_dns(&self) -> Result<IpAddress, Self::Error>;
    async fn ping(&self, host: &str) -> Result<Duration, Self::Error>;
}
```

### 方法说明

- `get_ip_address()` - 获取本机IP地址
- `get_gateway()` - 获取网关地址
- `get_dns()` - 获取DNS服务器地址
- `ping()` - Ping指定主机

### 关联类型

- `Error` - 连接操作错误类型

### 使用示例

```rust
let ip = wifi.get_ip_address().await?;
lxxcc::info!("IP: {}", ip);
```

## 数据类型定义

### WiFiNetwork

```rust
pub struct WiFiNetwork {
    pub ssid: String,
    pub rssi: i8,
    pub channel: u8,
    pub encryption: EncryptionType,
}
```

### EncryptionType

```rust
pub enum EncryptionType {
    Open,
    WEP,
    WPA,
    WPA2,
    WPA3,
}
```

### IpAddress

```rust
pub struct IpAddress {
    pub octets: [u8; 4],
}
```

## 实现注意事项

### ESP32-C6平台

- 使用`esp-radio`库驱动Wi-Fi
- 按需连接，完成同步后立即断开
- 正常电量：每2小时连接一次
- 低电量：每4小时连接一次
- 连接失败后2分30秒重试一次
- 低电量时仅重试1次
- 连接超时：30秒

### 泰山派平台

- 使用Linux Wi-Fi驱动
- 按需连接，完成同步后立即断开
- 正常电量：每2小时连接一次
- 低电量：每4小时连接一次
- 连接失败后2分30秒重试一次
- 低电量时仅重试1次
- 连接超时：30秒

### 模拟器平台

- 使用TUN/TAP网络接口
- 模拟Wi-Fi连接
- 模拟网络同步
- 连接超时：30秒

## 性能要求

- 初始化时间：≤100ms
- 连接时间：≤10秒
- 断开时间：≤1秒
- 扫描时间：≤5秒
- Ping延迟：≤100ms
- 信号强度精度：±3dBm
