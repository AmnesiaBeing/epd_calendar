# LPU (低功耗控制单元) 接口文档

## 接口概述

LPU (Low Power Unit) 是系统的低功耗控制单元，负责在主CPU深度睡眠时维持系统运行，并在必要时唤醒主CPU。在ESP32-C6平台上，LPU具体实现为LP Core低功耗核心。

## LPULifecycle Trait

### 接口定义

```rust
pub trait LPULifecycle {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn start(&mut self) -> Result<(), Self::Error>;
    async fn stop(&mut self) -> Result<(), Self::Error>;
    async fn reset(&mut self) -> Result<(), Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化LPU，配置低功耗模式和唤醒源
- `start()` - 启动LPU，开始低功耗监控
- `stop()` - 停止LPU，关闭低功耗监控
- `reset()` - 重置LPU，恢复初始状态

### 关联类型

- `Error` - LPU操作错误类型

### 使用示例

```rust
let mut lpu = Esp32C6LpuCore::new(peripherals.lp_core);
lpu.initialize().await?;
lpu.start().await?;
```

## LPUMonitor Trait

### 接口定义

```rust
pub trait LPUMonitor {
    type Error;

    async fn get_heartbeat(&self) -> Result<bool, Self::Error>;
    async fn get_sensor_data(&self) -> Result<SensorData, Self::Error>;
    async fn get_button_state(&self) -> Result<ButtonState, Self::Error>;
    async fn set_wakeup_schedule(&mut self, schedule: WakeupSchedule) -> Result<(), Self::Error>;
    async fn get_wakeup_schedule(&self) -> Result<WakeupSchedule, Self::Error>;
}
```

### 方法说明

- `get_heartbeat()` - 获取LPU心跳状态，确认LPU是否正常运行
- `get_sensor_data()` - 获取传感器数据（温湿度）
- `get_button_state()` - 获取按键状态（短按/长按）
- `set_wakeup_schedule()` - 设置唤醒计划，告诉LPU何时唤醒主CPU
- `get_wakeup_schedule()` - 获取当前唤醒计划

### 关联类型

- `Error` - 监控操作错误类型

### 使用示例

```rust
let heartbeat = lpu.get_heartbeat().await?;
if heartbeat {
    let sensor_data = lpu.get_sensor_data().await?;
    lxx_common::info!("Temperature: {}°C, Humidity: {}%", sensor_data.temp, sensor_data.humidity);
}
```

## LPUWake Trait

### 接口定义

```rust
pub trait LPUWake {
    type Error;

    async fn trigger_wakeup(&mut self) -> Result<(), Self::Error>;
    async fn cancel_wakeup(&mut self) -> Result<(), Self::Error>;
    async fn is_wakeup_pending(&self) -> Result<bool, Self::Error>;
}
```

### 方法说明

- `trigger_wakeup()` - 触发LPU唤醒主CPU
- `cancel_wakeup()` - 取消待处理的唤醒请求
- `is_wakeup_pending()` - 检查是否有待处理的唤醒请求

### 关联类型

- `Error` - 唤醒操作错误类型

### 使用示例

```rust
lpu.trigger_wakeup().await?;
```

## LPUSharedMemory Trait

### 接口定义

```rust
pub trait LPUSharedMemory {
    type Error;

    async fn read_shared_data(&self) -> Result<SharedMemoryData, Self::Error>;
    async fn write_shared_data(&mut self, data: SharedMemoryData) -> Result<(), Self::Error>;
    async fn sync_with_lpu(&mut self) -> Result<(), Self::Error>;
}
```

### 方法说明

- `read_shared_data()` - 从共享内存读取数据
- `write_shared_data()` - 向共享内存写入数据
- `sync_with_lpu()` - 与LPU同步共享内存

### 关联类型

- `Error` - 共享内存操作错误类型

### 使用示例

```rust
let data = lpu.read_shared_data().await?;
lpu.write_shared_data(new_data).await?;
lpu.sync_with_lpu().await?;
```

## 实现注意事项

### ESP32-C6平台

- LPU实现为LP Core，需要使用`esp-lp-hal`库
- 共享内存必须标记为RTC内存，使用`#[link_section = ".rtc_data"]`
- LP Core独立编译，需要单独的Cargo.toml配置
- 唤醒主CPU通过GPIO中断实现

### 泰山派平台

- LPU实现为低优先级线程
- 共享内存使用普通内存
- 唤醒主CPU通过信号量实现

### 模拟器平台

- LPU实现为模拟线程
- 共享内存使用内存缓冲区
- 唤醒主CPU通过条件变量实现

## 性能要求

- LPU心跳检测延迟：≤1秒
- 传感器读取延迟：≤100ms
- 按键检测延迟：≤100ms
- 共享内存读写延迟：≤10ms
- 唤醒触发延迟：≤50ms
