# 显示服务 (Display Service) 接口文档

## 接口概述

显示服务负责墨水屏驱动和UI渲染，优化刷新流程以减少主CPU占用时间。

## DisplayService Trait

```rust
pub trait DisplayService {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;
    async fn update_display(&mut self, data: DisplayData) -> Result<(), Self::Error>;
    async fn refresh(&mut self) -> Result<(), Self::Error>;
    async fn get_refresh_state(&self) -> Result<RefreshState, Self::Error>;
}
```

## Renderer Trait

```rust
pub trait Renderer {
    type Error;

    async fn render_time(&mut self, time: DateTime, lunar: LunarDate) -> Result<(), Self::Error>;
    async fn render_weather(&mut self, weather: WeatherInfo) -> Result<(), Self::Error>;
    async fn render_quote(&mut self, quote: Quote) -> Result<(), Self::Error>;
    async fn render_qrcode(&mut self, data: &str) -> Result<(), Self::Error>;
}
```

## DisplayManager Trait

```rust
pub trait DisplayManager {
    type Error;

    async fn schedule_refresh(&mut self) -> Result<(), Self::Error>;
    async fn cancel_refresh(&mut self) -> Result<(), Self::Error>;
    async fn get_refresh_queue(&self) -> Result<Vec<RefreshRequest>, Self::Error>;
}
```

## 性能要求

- 初始化时间：≤500ms
- 显示更新延迟：≤100ms
- 刷新完成时间：≤10秒
