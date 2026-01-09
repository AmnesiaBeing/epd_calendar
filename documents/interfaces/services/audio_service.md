# 音频服务 (Audio Service) 接口文档

## 接口概述

音频服务负责蜂鸣器控制，低电量下禁用非必要音频输出。

## AudioService Trait

```rust
pub trait AudioService {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn play_hour_chime(&mut self) -> Result<(), Self::Error>;
    async fn play_alarm(&mut self, melody: Melody) -> Result<(), Self::Error>;
    async fn stop(&mut self) -> Result<(), Self::Error>;
}
```

## 性能要求

- 初始化时间：≤10ms
- 整点报时延迟：≤500ms
- 闹钟播放延迟：≤100ms
