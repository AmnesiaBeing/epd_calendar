//! 平台抽象接口

use cfg_if::cfg_if;
use embassy_sync::channel::{Receiver, Sender};

cfg_if! {
    if #[cfg(feature = "esp32c6")] {
        // ESP32平台实现
        pub use lxx_calendar_boards_esp32c6::*;
    }
    else if #[cfg(feature = "simulator")] {
        // 模拟器平台实现
        pub use simulator_impl::*;
    }
    else if #[cfg(feature = "tspi")] {
        // TSPI平台实现
        pub use tspi_impl::*;
    }
    else {
        compile_error!("You need to select a platform feature: esp32c6, simulator, or tspi");
    }
}

const CAP: usize = 10;

/// 通道类型
pub type LxxChannel<T> = embassy_sync::channel::Channel<LxxAsyncMutex, T, CAP>;

/// 通道接收者类型
pub type LxxChannelReceiver<'a, T> = Receiver<'a, LxxAsyncMutex, T, CAP>;

/// 通道发送者类型
pub type LxxChannelSender<'a, T> = Sender<'a, LxxAsyncMutex, T, CAP>;
