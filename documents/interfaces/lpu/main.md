# LP Core 主接口文档

## 接口概述

LP Core主接口定义了LP Core的主函数和主要任务。LP Core是ESP32-C6的低功耗核心，负责低功耗监控和精确计时。

## LPUMain Trait

### 接口定义

```rust
pub trait LPUMain {
    type Error;

    fn main() -> !;
}
```

### 方法说明

- `main()` - LP Core主函数，永不返回

### 关联类型

- `Error` - LP Core错误类型

### 使用示例

```rust
#[entry]
fn main() -> ! {
    let mut lp_core = LpuCoreImpl::new();
    lp_core.initialize();
    lp_core.run();
}
```

## LPUMonitorTask Trait

### 接口定义

```rust
pub trait LPUMonitorTask {
    type Error;

    async fn run(&mut self) -> Result<(), Self::Error>;
    async fn check_time_events(&mut self) -> Result<(), Self::Error>;
    async fn read_sensors(&mut self) -> Result<(), Self::Error>;
    async fn monitor_button(&mut self) -> Result<(), Self::Error>;
}
```

### 方法说明

- `run()` - 运行监控任务主循环
- `check_time_events()` - 检查时间事件
- `read_sensors()` - 读取传感器数据
- `monitor_button()` - 监控按键状态

### 关联类型

- `Error` - 监控任务错误类型

### 使用示例

```rust
let mut monitor = LpuMonitorImpl::new();
monitor.run().await?;
```

## LPUWakeTask Trait

### 接口定义

```rust
pub trait LPUWakeTask {
    type Error;

    async fn schedule_wakeup(&mut self, time: u64, reason: WakeupReason) -> Result<(), Self::Error>;
    async fn trigger_wakeup(&mut self) -> Result<(), Self::Error>;
    async fn get_next_wakeup(&self) -> Result<Option<u64>, Self::Error>;
}
```

### 方法说明

- `schedule_wakeup()` - 调度唤醒
- `trigger_wakeup()` - 触发唤醒
- `get_next_wakeup()` - 获取下次唤醒时间

### 关联类型

- `Error` - 唤醒任务错误类型

### 使用示例

```rust
let mut wake_task = LpuWakeTaskImpl::new();
wake_task.schedule_wakeup(1705315200, WakeupReason::Timer).await?;
```

## 实现注意事项

### LP Core特性

- 独立的RISC-V核心
- 运行在RTC内存中
- 功耗：约10μA
- 时钟频率：可配置，默认低频

### 任务调度

- 每秒检查时间事件
- 每分钟读取传感器
- 持续监控按键
- 按需唤醒主CPU

### 共享内存

- 通过RTC共享内存与主CPU通信
- 使用原子操作避免锁竞争
- 数据更新后立即通知主CPU

### 唤醒机制

- 通过GPIO中断唤醒主CPU
- 支持定时器唤醒
- 支持按键唤醒
- 支持闹钟唤醒

## 性能要求

- 心跳检测延迟：≤1秒
- 传感器读取延迟：≤100ms
- 按键检测延迟：≤100ms
- 唤醒触发延迟：≤50ms
- 共享内存读写延迟：≤10ms
