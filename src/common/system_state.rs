use sxtwl_rs::{
    culture::{Taboo, Week, Zodiac},
    festival::{LunarFestival, SolarFestival},
    holiday::LegalHoliday,
    lunar::LunarDay,
    sixtycycle::SixtyCycle,
    solar::{SolarDay, SolarTerm},
};

use crate::{
    assets::generated_hitokoto_data::{HITOKOTOS, Hitokoto},
    common::weather::DailyWeather,
};

pub struct SystemState {
    pub time: TimeData,
    pub date: DateData,
    pub lunar: LunarData,
    pub weather: WeatherData, // 天气相关代码过长，统一放置在weather.rs中
    pub quote: &'static Hitokoto,
    pub charging_status: ChargingStatus,
    pub battery_level: BatteryLevel,
    pub network_status: NetworkStatus,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            time: TimeData::default(),
            date: DateData {
                day: SolarDay::new(2025, 12, 21).unwrap(),
                week: Week::from_index(0),
                holiday: None,
                festival: None,
            },
            lunar: LunarData::default(),
            weather: WeatherData::default(),
            quote: &HITOKOTOS[0],
            charging_status: ChargingStatus::default(),
            battery_level: BatteryLevel::default(),
            network_status: NetworkStatus::default(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ChargingStatus(pub bool);

#[derive(Default, Debug, Clone)]
pub struct NetworkStatus(pub bool);

#[derive(Debug, Clone, Default)]
pub struct TimeData {
    pub hour: u8,
    pub minute: u8,
    pub am_pm: Option<AMPM>, // 默认为None
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AMPM(pub bool); // false: am, true: pm

#[derive(Debug, Clone)]
pub struct DateData {
    pub day: SolarDay, // 该结构体内自带年、月
    pub week: Week,
    pub holiday: Option<LegalHoliday>,   // 节假日
    pub festival: Option<SolarFestival>, // 阳历节日
}

#[derive(Debug, Clone)]
pub struct LunarData {
    pub ganzhi: SixtyCycle,                      // 农历年干支（60个周期）
    pub zodiac: Zodiac,                          // 生肖
    pub day: LunarDay,                           // 农历日名称，该结构体内自带年、月
    pub jieqi: Option<SolarTerm>,                // 节气
    pub day_recommends: heapless::Vec<Taboo, 8>, // 适宜的事件
    pub day_avoids: heapless::Vec<Taboo, 8>,     // 避免的事件
    pub festival: Option<LunarFestival>,         // 农历节日
}

impl Default for LunarData {
    fn default() -> Self {
        Self {
            ganzhi: SixtyCycle::from_name("乙巳"),
            zodiac: Zodiac::from_name("蛇"),
            day: LunarDay::new(2025, 11, 2).unwrap(), // 农历十一月初二
            jieqi: Some(SolarTerm::from_name(2025, "冬至")),
            day_recommends: heapless::Vec::from_slice(&[
                Taboo::from_name("交易"),
                Taboo::from_name("进人口"),
                Taboo::from_name("祭祀"),
                Taboo::from_name("沐浴"),
                Taboo::from_name("捕捉"),
                Taboo::from_name("入殓"),
                Taboo::from_name("除服"),
                Taboo::from_name("成服"),
            ])
            .unwrap(),
            day_avoids: heapless::Vec::from_slice(&[
                Taboo::from_name("斋醮"),
                Taboo::from_name("入宅"),
                Taboo::from_name("修造"),
                Taboo::from_name("动土"),
                Taboo::from_name("破土"),
            ])
            .unwrap(),
            festival: None,
        }
    }
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

#[derive(Debug, Clone, Default)]
pub struct WeatherData {
    /// 温湿度传感器的数据
    pub sensor_data: Option<(f32, f32)>,
    /// 天气数据的位置及3天的预报数据
    pub daily_forecast: Option<(heapless::String<20>, [DailyWeather; 3])>,
}
