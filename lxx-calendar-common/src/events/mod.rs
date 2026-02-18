//! 系统事件定义模块
//! 
//! 本模块定义了系统所有的核心事件类型，包括：
//! - 唤醒事件 (WakeupEvent)
//! - 用户输入事件 (UserEvent)
//! - 时间事件 (TimeEvent)
//! - 网络事件 (NetworkEvent)
//! - 系统状态事件 (SystemStateEvent)
//! - 电源事件 (PowerEvent)
//! 
//! 所有事件都实现了Debug、Clone和Eq trait，便于日志记录和状态转换判断。

pub mod system;
pub use system::{SystemEvent, WakeupEvent, UserEvent, TimeEvent, NetworkEvent, SystemStateEvent, PowerEvent};
