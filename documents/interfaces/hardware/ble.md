# BLE (蓝牙低功耗) 接口文档

## 接口概述

BLE接口负责管理BLE（Bluetooth Low Energy）连接，用于配网、配置管理和OTA升级。BLE按需开启，低电量时禁用。

## BLEDriver Trait

### 接口定义

```rust
pub trait BLEDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn start_advertising(&mut self) -> Result<(), Self::Error>;
    async fn stop_advertising(&mut self) -> Result<(), Self::Error>;
    async fn is_advertising(&self) -> Result<bool, Self::Error>;
    async fn is_connected(&self) -> Result<bool, Self::Error>;
    async fn disconnect(&mut self) -> Result<(), Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化BLE模块
- `start_advertising()` - 开始广播
- `stop_advertising()` - 停止广播
- `is_advertising()` - 检测是否正在广播
- `is_connected()` - 检测是否已连接
- `disconnect()` - 断开BLE连接

### 关联类型

- `Error` - BLE操作错误类型

### 使用示例

```rust
let mut ble = Esp32C6Ble::new(radio);
ble.initialize().await?;
ble.start_advertising().await?;
if ble.is_connected().await? {
    lxxcc::info!("Device connected");
}
```

## BLEServer Trait

### 接口定义

```rust
pub trait BLEServer {
    type Error;

    async fn add_service(&mut self, service: BLEService) -> Result<(), Self::Error>;
    async fn remove_service(&mut self, uuid: &str) -> Result<(), Self::Error>;
    async fn get_services(&self) -> Result<Vec<BLEService>, Self::Error>;
    async fn notify(&mut self, characteristic_uuid: &str, data: &[u8]) -> Result<(), Self::Error>;
}
```

### 方法说明

- `add_service()` - 添加BLE服务
- `remove_service()` - 移除BLE服务
- `get_services()` - 获取所有BLE服务
- `notify()` - 向特征值发送通知

### 关联类型

- `Error` - BLE服务操作错误类型

### 使用示例

```rust
let service = BLEService {
    uuid: "0000180a-0000-1000-8000-00805f9b34fb".to_string(),
    characteristics: vec![],
};
ble.add_service(service).await?;
```

## 数据类型定义

### BLEService

```rust
pub struct BLEService {
    pub uuid: String,
    pub characteristics: Vec<BLECharacteristic>,
}
```

### BLECharacteristic

```rust
pub struct BLECharacteristic {
    pub uuid: String,
    pub properties: CharacteristicProperties,
    pub value: Vec<u8>,
}
```

### CharacteristicProperties

```rust
pub struct CharacteristicProperties {
    pub read: bool,
    pub write: bool,
    pub notify: bool,
    pub indicate: bool,
}
```

## 实现注意事项

### ESP32-C6平台

- 使用`esp-radio`库驱动BLE
- 正常电量下按需开启
- 低电量下禁用
- 未配网时快闪LED（2Hz）
- 已配网时慢闪LED（0.5Hz）
- 超时机制：5分钟无操作自动退出
- 加密通信：使用RSA硬件加密解密
- 配置接收：Wi-Fi、闹钟、时区等
- OTA升级：接收固件、校验完整性、写入备用分区

### 泰山派平台

- 使用Linux BLE驱动
- 正常电量下按需开启
- 低电量下禁用
- 未配网时快闪LED（2Hz）
- 已配网时慢闪LED（0.5Hz）
- 超时机制：5分钟无操作自动退出
- 加密通信：使用软件加密解密
- 配置接收：Wi-Fi、闹钟、时区等
- OTA升级：接收固件、校验完整性、写入备用分区

### 模拟器平台

- 使用虚拟BLE模拟
- 正常电量下按需开启
- 低电量下禁用
- 未配网时快闪LED（2Hz）
- 已配网时慢闪LED（0.5Hz）
- 超时机制：5分钟无操作自动退出
- 加密通信：跳过加密，返回原始数据
- 配置接收：Wi-Fi、闹钟、时区等
- OTA升级：模拟固件接收、校验、分区切换

## 性能要求

- 初始化时间：≤100ms
- 广播启动时间：≤100ms
- 广播停止时间：≤100ms
- 连接建立时间：≤5秒
- 断开时间：≤1秒
- 通知发送延迟：≤10ms
- 加密/解密延迟：≤100ms
