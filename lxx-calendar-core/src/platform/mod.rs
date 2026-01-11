//! 平台抽象接口
//! 这个模块使用严格的类型抽象来管理不同平台的实现

use cfg_if::cfg_if;
use embassy_executor::Spawner;

// 重新导出traits中的平台定义
pub use crate::traits::{Platform, PlatformAsyncTypes};

/// 硬件上下文类型
pub type HwiContext = ();

// 根据不同的平台feature导出不同的平台实现
// 与printhor项目类似，但暂时在内部实现简单的平台支持
// 这样可以避免依赖不存在的外部库，同时保持代码结构的正确性
cfg_if! {
    if #[cfg(feature = "esp32c6")] {
        // ESP32平台实现
        // 这里我们直接在内部实现一个简单的ESP32平台支持
        // 实际项目中应该替换为外部平台库
        pub mod esp32_impl;
        pub use esp32_impl::*;
    }
    else if #[cfg(feature = "simulator")] {
        // 模拟器平台实现
        pub mod simulator_impl;
        pub use simulator_impl::*;
    }
    else if #[cfg(feature = "tspi")] {
        // TSPI平台实现
        pub mod tspi_impl;
        pub use tspi_impl::*;
    }
    else {
        compile_error!("You need to select a platform feature: esp32c6, simulator, or tspi");
    }
}