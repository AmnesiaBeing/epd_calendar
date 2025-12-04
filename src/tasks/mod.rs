// src/tasks/mod.rs
pub mod display_task;
pub mod quote_task;
pub mod status_task;
pub mod time_task;
pub mod weather_task;

pub use display_task::display_task;
pub use quote_task::quote_task;
pub use status_task::status_task;
pub use time_task::time_task;
pub use weather_task::weather_task;

use embassy_sync::channel::Channel;

use core::fmt::Debug;

use crate::{
    assets::generated_hitokoto_data::Hitokoto,
    common::{
        BatteryLevel, ChargingStatus, DateData, GlobalChannel, NetworkStatus, TimeData, WeatherData,
    },
};

/// 显示事件 - 简化版本，只保留实际使用的类型
#[derive(Debug)]
pub enum DisplayEvent {
    /// 更新特定组件（包含组件数据）
    UpdateComponent(ComponentDataType),
    /// 强制全屏刷新（用于系统重置或错误恢复）
    #[allow(unused)]
    ForceFullRefresh,
}

#[derive(Debug)]
pub enum ComponentDataType {
    TimeType(TimeData),
    DateType(DateData),
    WeatherType(WeatherData),
    QuoteType(&'static Hitokoto),
    BatteryType(BatteryLevel),
    ChargingStatusType(ChargingStatus),
    NetworkStatusType(NetworkStatus),
}

impl Debug for Hitokoto {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let hitokoto = self.hitokoto;
        write!(f, "Hitokoto: {}", hitokoto)
    }
}

// 全局事件通道
pub static DISPLAY_EVENTS: GlobalChannel<DisplayEvent> = Channel::new();
