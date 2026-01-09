# 共享内存 (Shared Memory) 数据类型文档

## 概述

共享内存用于LPU和主CPU之间的数据交换。在ESP32-C6平台上，共享内存必须标记为RTC内存，使用`#[link_section = ".rtc_data"]`。

## SharedMemoryLayout

### 结构定义

```rust
#[repr(C)]
pub struct SharedMemoryLayout {
    pub magic: u32,
    pub version: u32,
    pub flags: SharedFlags,
    pub wakeup_schedule: WakeupSchedule,
    pub sensor_data: SensorData,
    pub button_state: ButtonState,
    pub system_state: SystemState,
    pub reserved: [u8; 128],
}
```

### 字段说明

- `magic` - 魔数，用于验证共享内存有效性（0x4C50554C）
- `version` - 版本号，用于兼容性检查
- `flags` - 共享标志位
- `wakeup_schedule` - 唤醒计划
- `sensor_data` - 传感器数据
- `button_state` - 按键状态
- `system_state` - 系统状态
- `reserved` - 预留空间

## WakeupSchedule

### 结构定义

```rust
#[repr(C)]
pub struct WakeupSchedule {
    pub next_wakeup_time: u64,
    pub wakeup_reason: WakeupReason,
    pub scheduled_tasks: ScheduledTasks,
}
```

### 字段说明

- `next_wakeup_time` - 下次唤醒时间（Unix时间戳，秒）
- `wakeup_reason` - 唤醒原因
- `scheduled_tasks` - 调度的任务

## SensorData

### 结构定义

```rust
#[repr(C)]
pub struct SensorData {
    pub temperature: i16,
    pub humidity: u16,
    pub timestamp: u64,
    pub valid: bool,
}
```

### 字段说明

- `temperature` - 温度（0.01°C单位，例如：2250表示22.50°C）
- `humidity` - 湿度（0.01%单位，例如：6500表示65.00%）
- `timestamp` - 数据时间戳（Unix时间戳，秒）
- `valid` - 数据是否有效

## ButtonState

### 结构定义

```rust
#[repr(C)]
pub struct ButtonState {
    pub is_pressed: bool,
    pub press_duration: u32,
    pub last_press_time: u64,
}
```

### 字段说明

- `is_pressed` - 按键是否按下
- `press_duration` - 按键持续时间（毫秒）
- `last_press_time` - 上次按键时间（Unix时间戳，秒）

## SystemState

### 结构定义

```rust
#[repr(C)]
pub struct SystemState {
    pub current_state: SystemMode,
    pub battery_level: u8,
    pub last_sync_time: u64,
}
```

### 字段说明

- `current_state` - 当前系统状态
- `battery_level` - 电池电量（0-100%）
- `last_sync_time` - 上次同步时间（Unix时间戳，秒）

## 数据类型定义

### SharedFlags

```rust
#[repr(C)]
pub struct SharedFlags {
    pub lpu_initialized: bool,
    pub main_cpu_initialized: bool,
    pub data_dirty: bool,
    pub reserved: u8,
}
```

### WakeupReason

```rust
#[repr(C)]
pub enum WakeupReason {
    None,
    Timer,
    Button,
    Alarm,
    NetworkSync,
    DisplayRefresh,
}
```

### SystemMode

```rust
#[repr(C)]
pub enum SystemMode {
    DeepSleep,
    BleConnection,
    NormalWork,
}
```

### ScheduledTasks

```rust
#[repr(C)]
pub struct ScheduledTasks {
    pub display_refresh: bool,
    pub network_sync: bool,
    pub alarm_check: bool,
    pub reserved: u8,
}
```

## 内存布局

### ESP32-C6平台

```
RTC内存（8KB）：
- LP Core程序代码：4KB（0x0000-0x0FFF）
- LP Core数据/堆栈：1KB（0x1000-0x13FF）
- 共享数据：2KB（0x1400-0x1BFF）
- 预留：1KB（0x1C00-0x1FFF）
```

### 其他平台

```
普通内存：
- 共享数据：2KB
- 预留：1KB
```

## 使用示例

### ESP32-C6平台

```rust
#[link_section = ".rtc_data"]
static mut SHARED_MEMORY: SharedMemoryLayout = SharedMemoryLayout {
    magic: 0x4C50554C,
    version: 1,
    flags: SharedFlags { lpu_initialized: false, main_cpu_initialized: false, data_dirty: false, reserved: 0 },
    wakeup_schedule: WakeupSchedule { next_wakeup_time: 0, wakeup_reason: WakeupReason::None, scheduled_tasks: ScheduledTasks { display_refresh: false, network_sync: false, alarm_check: false, reserved: 0 } },
    sensor_data: SensorData { temperature: 0, humidity: 0, timestamp: 0, valid: false },
    button_state: ButtonState { is_pressed: false, press_duration: 0, last_press_time: 0 },
    system_state: SystemState { current_state: SystemMode::DeepSleep, battery_level: 100, last_sync_time: 0 },
    reserved: [0u8; 128],
};

fn read_shared_memory() -> &'static SharedMemoryLayout {
    unsafe { &SHARED_MEMORY }
}

fn write_shared_memory(data: &SharedMemoryLayout) {
    unsafe {
        SHARED_MEMORY = *data;
    }
}
```

### 其他平台

```rust
static mut SHARED_MEMORY: SharedMemoryLayout = SharedMemoryLayout { ... };

fn read_shared_memory() -> &'static SharedMemoryLayout {
    unsafe { &SHARED_MEMORY }
}

fn write_shared_memory(data: &SharedMemoryLayout) {
    unsafe {
        SHARED_MEMORY = *data;
    }
}
```

## 注意事项

### ESP32-C6平台

- 共享内存必须使用`#[link_section = ".rtc_data"]`标记
- 共享内存大小限制为2KB
- 访问共享内存时需要使用`unsafe`代码块
- 避免频繁写入共享内存，减少功耗

### 其他平台

- 共享内存使用普通内存
- 共享内存大小限制为2KB
- 访问共享内存时需要使用`unsafe`代码块
- 避免频繁写入共享内存，减少功耗

### 通用注意事项

- 使用原子操作避免数据竞争
- 写入共享内存后立即同步
- 读取共享内存前检查magic和version
- 数据更新时设置data_dirty标志
