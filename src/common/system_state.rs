use sxtwl_rs::{
    culture::{Taboo, Zodiac},
    festival::{LunarFestival, SolarFestival},
    holiday::LegalHoliday,
    lunar::{LunarDay, LunarMonth},
    sixtycycle::SixtyCycle,
    solar::SolarDay,
};

use crate::{assets::generated_hitokoto_data::Hitokoto, common::weather::WeatherData};

#[derive(Default)]
pub struct SystemState {
    pub time: Option<TimeData>,
    pub date: Option<DateData>,
    pub weather: Option<WeatherData>, // 天气相关代码过长，统一放置在weather.rs中
    pub quote: Option<&'static Hitokoto>,
    pub is_charging: ChargingStatus,
    pub battery_level: BatteryLevel,
    pub is_online: NetworkStatus,
}

#[derive(Default, Debug, Clone)]
pub struct ChargingStatus(pub bool);

#[derive(Default, Debug, Clone)]
pub struct NetworkStatus(pub bool);

#[derive(Debug, Clone)]
pub struct TimeData {
    pub hour: u8,
    pub minute: u8,
}

#[derive(Debug, Clone)]
pub struct DateData {
    pub day: SolarDay, // 该结构体内自带年、月
    pub week: u8,
    pub holiday: Option<LegalHoliday>,   // 节假日
    pub festival: Option<SolarFestival>, // 阳历节日
    pub lunar: Option<LunarData>,
}

#[derive(Debug, Clone)]
pub struct LunarData {
    pub ganzhi: SixtyCycle,                      // 农历年干支（60个周期）
    pub zodiac: Zodiac,                          // 生肖
    pub month: LunarMonth,                       // 农历月名称，注意闰月信息，负数表示闰月
    pub day: LunarDay,                           // 农历日名称
    pub jieqi: Option<u8>,                       // 节气
    pub day_recommends: heapless::Vec<Taboo, 8>, // 适宜的事件
    pub day_avoids: heapless::Vec<Taboo, 8>,     // 避免的事件
    pub festival: Option<LunarFestival>,         // 农历节日
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum BatteryLevel {
    #[default]
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
}
