// src/driver/key/mod.rs
//! 按键驱动模块
//! 支持ESP32（中断触发）、Tspi（泰山派，轮询/中断）平台，模拟器仅空实现

use crate::common::GlobalSignal;
use crate::{common::error::Result, platform::Platform};

/// 长按判定阈值（可根据需求调整，单位：毫秒）
pub const LONG_PRESS_THRESHOLD_MS: u64 = 1000;

/// 按键事件枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    /// 短按事件
    ShortPress,
    /// 长按事件
    LongPress,
}

/// 按键驱动通用接口
pub trait KeyDriver<'a> {
    type P: Platform;

    /// 创建按键驱动实例
    /// - `peripherals`: 平台外设
    /// - `short_press_sig`: 短按事件信号
    /// - `long_press_sig`: 长按事件信号
    fn create(
        peripherals: &'a mut <Self::P as Platform>::Peripherals,
        short_press_sig: GlobalSignal<KeyEvent>,
        long_press_sig: GlobalSignal<KeyEvent>,
    ) -> Result<Self>
    where
        Self: Sized;

    /// 启动按键监听（ESP32：启动中断；Tspi：启动轮询任务）
    fn start(&mut self) -> Result<()>;
}

// 平台条件编译模块
#[cfg(feature = "esp32")]
mod esp32;
#[cfg(feature = "simulator")]
mod simulator;
#[cfg(feature = "tspi")]
mod tspi;

// 平台默认驱动类型别名
#[cfg(feature = "esp32")]
pub type DefaultKeyDriver<'a> = esp32::Esp32KeyDriver<'a>;
#[cfg(feature = "tspi")]
pub type DefaultKeyDriver<'a> = tspi::TspiKeyDriver<'a>;
#[cfg(feature = "simulator")]
pub type DefaultKeyDriver<'a> = simulator::SimulatorKeyDriver<'a>;
