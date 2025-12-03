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
        ChargingStatus, GlobalChannel, NetworkStatus,
        system_state::{BatteryLevel, DateData, TimeData},
        weather::WeatherData,
    },
};

// 显示事件枚举
pub enum DisplayEvent {
    // 全局刷新
    FullRefresh,
    // 部分刷新
    PartialRefresh(PartialRefreshType),
    // 更新特定组件
    UpdateComponent(ComponentData),
    // 请求重新计算农历
    RequestLunarCalc,
}

#[derive(Debug, Clone)]
pub enum PartialRefreshType {
    TimeOnly,
    DateOnly,
    WeatherOnly,
    QuoteOnly,
    StatusOnly,
    TimeAndDate,
}

#[derive(Debug)]
pub enum ComponentData {
    TimeData(TimeData),
    DateData(DateData),
    WeatherData(WeatherData),
    QuoteData(&'static Hitokoto),
    BatteryData(BatteryLevel),
    ChargingStatus(ChargingStatus),
    NetworkStatus(NetworkStatus),
}

impl Debug for Hitokoto {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Hitokoto: {}", self.hitokoto)
    }
}

// 全局事件通道
pub static DISPLAY_EVENTS: GlobalChannel<DisplayEvent> = Channel::new();
