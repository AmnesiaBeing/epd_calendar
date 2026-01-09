# 系统事件 (System Events) 文档

## 概述

系统事件用于服务间通信，基于`embassy-sync`异步通道实现。事件驱动架构确保模块解耦，主CPU任务处理完成后立即触发休眠。

## SystemEvent

### 枚举定义

```rust
pub enum SystemEvent {
    WakeupEvent(WakeupEvent),
    UserEvent(UserEvent),
    TimeEvent(TimeEvent),
    NetworkEvent(NetworkEvent),
    SystemEvent(SystemEvent),
    PowerEvent(PowerEvent),
}
```

### 变体说明

- `WakeupEvent` - 唤醒事件
- `UserEvent` - 用户输入事件
- `TimeEvent` - 时间事件
- `NetworkEvent` - 网络事件
- `SystemEvent` - 系统事件
- `PowerEvent` - 电源事件

### 使用示例

```rust
match event {
    SystemEvent::WakeupEvent(evt) => lxxcc::info!("Wakeup: {:?}", evt),
    SystemEvent::UserEvent(evt) => lxxcc::info!("User: {:?}", evt),
    SystemEvent::TimeEvent(evt) => lxxcc::info!("Time: {:?}", evt),
    SystemEvent::NetworkEvent(evt) => lxxcc::info!("Network: {:?}", evt),
    SystemEvent::SystemEvent(evt) => lxxcc::info!("System: {:?}", evt),
    SystemEvent::PowerEvent(evt) => lxxcc::info!("Power: {:?}", evt),
}
```

## WakeupEvent

### 枚举定义

```rust
pub enum WakeupEvent {
    WakeFromDeepSleep,
    WakeByLPU,
    WakeByButton,
    WakeByWDT,
}
```

### 变体说明

- `WakeFromDeepSleep` - 从深度睡眠唤醒
- `WakeByLPU` - LPU定时器唤醒
- `WakeByButton` - 按键唤醒
- `WakeByWDT` - 看门狗唤醒

### 使用示例

```rust
match wakeup_event {
    WakeupEvent::WakeFromDeepSleep => lxxcc::info!("Waking from deep sleep"),
    WakeupEvent::WakeByLPU => lxxcc::info!("Waking by LPU timer"),
    WakeupEvent::WakeByButton => lxxcc::info!("Waking by button"),
    WakeupEvent::WakeByWDT => lxxcc::info!("Waking by watchdog"),
}
```

## UserEvent

### 枚举定义

```rust
pub enum UserEvent {
    ButtonShortPress,
    ButtonLongPress,
    BLEConfigReceived(ConfigData),
}
```

### 变体说明

- `ButtonShortPress` - 按键短按
- `ButtonLongPress` - 按键长按15秒
- `BLEConfigReceived` - 收到蓝牙配置

### 使用示例

```rust
match user_event {
    UserEvent::ButtonShortPress => lxxcc::info!("Short press"),
    UserEvent::ButtonLongPress => lxxcc::info!("Long press"),
    UserEvent::BLEConfigReceived(config) => lxxcc::info!("Config received"),
}
```

## TimeEvent

### 枚举定义

```rust
pub enum TimeEvent {
    MinuteTick,
    HourChimeTrigger,
    AlarmTrigger(AlarmInfo),
}
```

### 变体说明

- `MinuteTick` - 每分钟触发（显示刷新）
- `HourChimeTrigger` - 整点报时触发（XX:59:56-XX:00:00）
- `AlarmTrigger` - 闹钟触发

### 使用示例

```rust
match time_event {
    TimeEvent::MinuteTick => lxxcc::info!("Minute tick"),
    TimeEvent::HourChimeTrigger => lxxcc::info!("Hour chime"),
    TimeEvent::AlarmTrigger(alarm) => lxxcc::info!("Alarm: {:?}", alarm),
}
```

## NetworkEvent

### 枚举定义

```rust
pub enum NetworkEvent {
    NetworkSyncRequested,
    NetworkSyncComplete(SyncResult),
    NetworkSyncFailed(NetworkError),
}
```

### 变体说明

- `NetworkSyncRequested` - 请求网络同步
- `NetworkSyncComplete` - 网络同步完成（含时间和天气）
- `NetworkSyncFailed` - 网络同步失败

### 使用示例

```rust
match network_event {
    NetworkEvent::NetworkSyncRequested => lxxcc::info!("Sync requested"),
    NetworkEvent::NetworkSyncComplete(result) => lxxcc::info!("Sync complete"),
    NetworkEvent::NetworkSyncFailed(err) => lxxcc::error!("Sync failed"),
}
```

## SystemEvent

### 枚举定义

```rust
pub enum SystemEvent {
    EnterDeepSleep,
    EnterBLEMode,
    EnterNormalMode,
    ConfigChanged(ConfigChange),
    LowPowerDetected,
    OTATriggered,
    OTAUpdateComplete,
}
```

### 变体说明

- `EnterDeepSleep` - 即将进入深度睡眠
- `EnterBLEMode` - 进入蓝牙模式
- `EnterNormalMode` - 进入正常工作模式
- `ConfigChanged` - 配置变更
- `LowPowerDetected` - 检测到低电量
- `OTATriggered` - 收到OTA升级指令
- `OTAUpdateComplete` - OTA升级完成

### 使用示例

```rust
match system_event {
    SystemEvent::EnterDeepSleep => lxxcc::info!("Entering deep sleep"),
    SystemEvent::EnterBLEMode => lxxcc::info!("Entering BLE mode"),
    SystemEvent::EnterNormalMode => lxxcc::info!("Entering normal mode"),
    SystemEvent::ConfigChanged(change) => lxxcc::info!("Config changed"),
    SystemEvent::LowPowerDetected => lxxcc::warn!("Low power detected"),
    SystemEvent::OTATriggered => lxxcc::info!("OTA triggered"),
    SystemEvent::OTAUpdateComplete => lxxcc::info!("OTA complete"),
}
```

## PowerEvent

### 枚举定义

```rust
pub enum PowerEvent {
    BatteryLevelChanged(u8),
    ChargingStateChanged(bool),
    LowPowerModeChanged(bool),
}
```

### 变体说明

- `BatteryLevelChanged` - 电池电量变化
- `ChargingStateChanged` - 充电状态变化
- `LowPowerModeChanged` - 低电量模式变化

### 使用示例

```rust
match power_event {
    PowerEvent::BatteryLevelChanged(level) => lxxcc::info!("Battery: {}%", level),
    PowerEvent::ChargingStateChanged(charging) => lxxcc::info!("Charging: {}", charging),
    PowerEvent::LowPowerModeChanged(enabled) => lxxcc::info!("Low power mode: {}", enabled),
}
```

## 数据类型定义

### ConfigData

```rust
pub struct ConfigData {
    pub wifi_ssid: String,
    pub wifi_password: String,
    pub weather_api_key: String,
    pub location_id: String,
    pub alarms: Vec<AlarmConfig>,
    pub timezone_offset: i32,
}
```

### SyncResult

```rust
pub struct SyncResult {
    pub time_synced: bool,
    pub weather_synced: bool,
    pub sync_duration: Duration,
}
```

### ConfigChange

```rust
pub enum ConfigChange {
    WiFiChanged,
    AlarmChanged,
    TimezoneChanged,
    DisplayThemeChanged,
}
```

### AlarmInfo

```rust
pub struct AlarmInfo {
    pub index: usize,
    pub time: TimeOfDay,
    pub repeat: Repeat,
}
```

## 实现注意事项

### 事件通道

- 使用`embassy-sync`的`channel`实现事件通道
- 支持优先级队列，高优先级事件优先处理
- 通道容量：32个事件
- 通道满时丢弃低优先级事件

### 事件分发

- 状态管理器作为事件消费者
- 各服务作为事件生产者
- 事件处理完成后立即触发主CPU休眠
- 避免事件堆积，减少唤醒次数

### 事件优先级

1. **最高优先级**：唤醒事件、电源事件
2. **高优先级**：用户事件、系统事件
3. **中优先级**：时间事件
4. **低优先级**：网络事件

### 事件过滤

- 低电量模式下过滤非核心事件
- 深度睡眠时过滤所有事件（唤醒事件除外）
- BLE模式下过滤网络事件

## 性能要求

- 事件发送延迟：≤1ms
- 事件接收延迟：≤1ms
- 事件处理延迟：≤10ms
- 通道容量：32个事件
- 事件丢失率：<0.1%
