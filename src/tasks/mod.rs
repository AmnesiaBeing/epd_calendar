// src/tasks/mod.rs

//! 任务模块 - 定义系统异步任务和事件处理机制
//!
//! 该模块包含显示任务、时间任务、天气任务、名言任务和状态任务等系统核心异步任务。

pub mod display_task;

pub use display_task::display_task;

use embassy_sync::channel::Channel;

use core::fmt::Debug;

/// 显示事件 - 简化版本，只保留实际使用的类型
#[derive(Debug)]
pub enum DisplayEvent {
    /// 更新特定组件（包含组件数据）
    UpdateComponent(ComponentDataType),
    /// 强制全屏刷新（用于系统重置或错误恢复）
    #[allow(unused)]
    ForceFullRefresh,
}

// /// 组件数据类型枚举，定义各种显示组件的数据类型
// #[derive(Debug)]
// pub enum ComponentDataType {
//     /// 时间组件数据
//     TimeType(TimeData),
//     /// 日期组件数据
//     DateType(DateData),
//     /// 天气组件数据
//     WeatherType(WeatherData),
//     /// 名言组件数据
//     QuoteType(&'static Hitokoto),
//     /// 电池电量组件数据
//     BatteryType(BatteryLevel),
//     /// 充电状态组件数据
//     ChargingStatusType(ChargingStatus),
//     /// 网络状态组件数据
//     NetworkStatusType(NetworkStatus),
// }

// impl Debug for Hitokoto {
//     /// 为Hitokoto结构体实现Debug trait
//     ///
//     /// # 参数
//     /// - `f`: 格式化器
//     ///
//     /// # 返回值
//     /// - `core::fmt::Result`: 格式化结果
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         let hitokoto = self.hitokoto;
//         write!(f, "Hitokoto: {}", hitokoto)
//     }
// }

// 全局事件通道
pub static DISPLAY_EVENTS: GlobalChannel<DisplayEvent> = Channel::new();
