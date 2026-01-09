# 时间服务 (Time Service) 接口文档

## 接口概述

时间服务负责时间管理、日历计算、事件调度，为LPU提供唤醒计划。

## TimeService Trait

```rust
pub trait TimeService {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn get_current_time(&self) -> Result<DateTime, Self::Error>;
    async fn set_time(&mut self, datetime: DateTime) -> Result<(), Self::Error>;
    async fn get_lunar_date(&self) -> Result<LunarDate, Self::Error>;
    async fn get_solar_term(&self) -> Result<Option<SolarTerm>, Self::Error>;
    async fn get_holiday(&self) -> Result<Option<Holiday>, Self::Error>;
    async fn calculate_wakeup_schedule(&self) -> Result<WakeupSchedule, Self::Error>;
}
```

## CalendarService Trait

```rust
pub trait CalendarService {
    type Error;

    async fn calculate_lunar(&self, date: DateTime) -> Result<LunarDate, Self::Error>;
    async fn get_solar_terms(&self, year: u16) -> Result<Vec<SolarTerm>, Self::Error>;
    async fn get_holidays(&self, year: u16) -> Result<Vec<Holiday>, Self::Error>;
}
```

## Scheduler Trait

```rust
pub trait Scheduler {
    type Error;

    async fn schedule_wakeup(&mut self, time: u64, reason: WakeupReason) -> Result<(), Self::Error>;
    async fn get_next_wakeup(&self) -> Result<Option<u64>, Self::Error>;
    async fn cancel_wakeup(&mut self) -> Result<(), Self::Error>;
}
```

## 性能要求

- 时间读取延迟：≤10ms
- 农历计算延迟：≤100ms
- 唤醒计划计算延迟：≤100ms
