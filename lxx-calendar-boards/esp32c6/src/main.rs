#![no_std]
#![no_main]

pub use lxx_calendar_core as lxx_core;
use lxx_calendar_common as lxx_common;

// 从common库中导入平台定义
// 这里会根据启用的feature自动导入对应的平台实现
use lxx_common::platform::{Platform, PlatformAsyncTypes, RawMutex};

use embassy_executor::Spawner;
use esp_rtos::main as platform_main;

#[platform_main]
async fn main(spawner: Spawner) {
    // 调用核心应用逻辑，不需要传递系统事件通道，由core_main内部创建
    lxx_core::core_main::<RawMutex>(spawner).await.unwrap();
}