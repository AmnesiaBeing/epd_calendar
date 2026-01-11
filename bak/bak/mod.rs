// src/tasks/mod.rs

//! 任务模块 - 定义系统异步任务和事件处理机制
//!
//! 该模块包含显示任务、时间任务、天气任务、名言任务和状态任务等系统核心异步任务。

pub mod main_task;

use embassy_sync::channel::Channel;

use core::fmt::Debug;

use crate::common::GlobalChannel;

/// 显示事件 - 简化版本，只保留实际使用的类型
#[derive(Debug)]
pub enum DisplayEvent {
    FullRefresh,
}

// 全局事件通道
pub static DISPLAY_EVENTS: GlobalChannel<DisplayEvent> = Channel::new();
