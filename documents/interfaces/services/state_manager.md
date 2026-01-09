# 状态管理器 (State Manager) 接口文档

## 接口概述

状态管理器负责维护系统状态机，处理事件分发和状态转换，保障主CPU深度休眠优先级。

## StateManager Trait

### 接口定义

```rust
pub trait StateManager {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn start(&mut self) -> Result<(), Self::Error>;
    async fn stop(&mut self) -> Result<(), Self::Error>;
    async fn handle_event(&mut self, event: SystemEvent) -> Result<(), Self::Error>;
    async fn get_current_state(&self) -> Result<SystemMode, Self::Error>;
    async fn transition_to(&mut self, mode: SystemMode) -> Result<(), Self::Error>;
}
```

### 方法说明

- `initialize()` - 初始化状态管理器
- `start()` - 启动状态管理器
- `stop()` - 停止状态管理器
- `handle_event()` - 处理系统事件
- `get_current_state()` - 获取当前系统状态
- `transition_to()` - 转换到指定状态

### 关联类型

- `Error` - 状态管理器错误类型

### 使用示例

```rust
let mut state_manager = StateManagerImpl::new(event_channel);
state_manager.initialize().await?;
state_manager.start().await?;
state_manager.handle_event(SystemEvent::WakeupEvent(WakeupEvent::WakeFromDeepSleep)).await?;
```

## StateTransition Trait

### 接口定义

```rust
pub trait StateTransition {
    type Error;

    async fn can_transition(&self, from: SystemMode, to: SystemMode) -> Result<bool, Self::Error>;
    async fn execute_transition(&mut self, from: SystemMode, to: SystemMode) -> Result<(), Self::Error>;
    async fn get_transition_history(&self) -> Result<Vec<TransitionRecord>, Self::Error>;
}
```

### 方法说明

- `can_transition()` - 检查是否可以转换状态
- `execute_transition()` - 执行状态转换
- `get_transition_history()` - 获取状态转换历史

### 关联类型

- `Error` - 状态转换错误类型

### 使用示例

```rust
if state_manager.can_transition(SystemMode::DeepSleep, SystemMode::NormalWork).await? {
    state_manager.execute_transition(SystemMode::DeepSleep, SystemMode::NormalWork).await?;
}
```

## 数据类型定义

### TransitionRecord

```rust
pub struct TransitionRecord {
    pub from: SystemMode,
    pub to: SystemMode,
    pub timestamp: u64,
    pub reason: String,
}
```

## 实现注意事项

### 状态转换规则

- 深度睡眠 → 蓝牙连接：LPU定时器/按键唤醒
- 深度睡眠 → 正常工作：LPU定时器唤醒
- 正常工作 → 蓝牙连接：按键短按
- 正常工作 → 深度睡眠：任务完成/到达休眠时间
- 蓝牙连接 → 正常工作：已配网+超时/未配网+超时
- 蓝牙连接 → 深度睡眠：未配网+超时

### 事件优先级

1. 唤醒事件：立即处理
2. 用户事件：立即处理
3. 系统事件：立即处理
4. 时间事件：按调度处理
5. 网络事件：按调度处理
6. 电源事件：立即处理

### 休眠触发

- 任务处理完成后立即触发休眠
- 无任何空闲逻辑
- 计算下次唤醒时间
- 同步唤醒计划到LPU

## 性能要求

- 初始化时间：≤100ms
- 事件处理延迟：≤10ms
- 状态转换延迟：≤100ms
- 休眠触发延迟：≤500ms
