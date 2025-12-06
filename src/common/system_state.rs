// src/common/system_state.rs

/// 系统状态管理模块
///
/// 本模块定义了EPD日历系统的核心状态数据结构
/// 包含时间、日期、农历、天气、名言警句、电池状态等系统状态信息
use sxtwl_rs::{
    culture::{Taboo, Week, Zodiac},
    festival::{LunarFestival, SolarFestival},
    holiday::LegalHoliday,
    lunar::LunarDay,
    sixtycycle::SixtyCycle,
    solar::{SolarDay, SolarTerm},
};

use crate::{
    assets::generated_hitokoto_data::HITOKOTOS, common::Hitokoto, common::weather::DailyWeather,
};

/// 系统状态结构体
///
/// 包含EPD日历系统的所有核心状态信息
pub struct SystemState {
    /// 时间数据
    pub time: TimeData,
    /// 日期数据
    pub date: DateData,
    /// 农历数据
    pub lunar: LunarData,
    /// 天气数据
    pub weather: WeatherData, // 天气相关代码过长，统一放置在weather.rs中
    /// 名言警句
    pub quote: &'static Hitokoto,
    /// 充电状态
    pub charging_status: ChargingStatus,
    /// 电池电量
    pub battery_level: BatteryLevel,
    /// 网络状态
    pub network_status: NetworkStatus,
}

impl Default for SystemState {
    /// 提供系统状态的默认实现
    ///
    /// 返回一个包含默认值的系统状态实例
    /// 主要用于测试和初始化
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

/// 充电状态结构体
///
/// 表示设备的充电状态
/// - true: 正在充电
/// - false: 未充电
#[derive(Default, Debug, Clone)]
pub struct ChargingStatus(pub bool);

/// 网络状态结构体
///
/// 表示设备的网络连接状态
/// - true: 已连接
/// - false: 未连接
#[derive(Default, Debug, Clone)]
pub struct NetworkStatus(pub bool);

/// 时间数据结构体
///
/// 包含小时、分钟和AM/PM信息
#[derive(Debug, Clone, Default)]
pub struct TimeData {
    /// 小时 (0-23)
    pub hour: u8,
    /// 分钟 (0-59)
    pub minute: u8,
    /// AM/PM指示器 (None表示24小时制)
    pub am_pm: Option<AMPM>, // 默认为None
}

/// AM/PM结构体
///
/// 表示上午/下午时间
/// - false: AM (上午)
/// - true: PM (下午)
#[derive(Debug, Clone, Copy, Default)]
pub struct AMPM(pub bool); // false: am, true: pm

/// 日期数据结构体
///
/// 包含阳历日期、星期、节假日和节日信息
#[derive(Debug, Clone)]
pub struct DateData {
    /// 阳历日期
    pub day: SolarDay, // 该结构体内自带年、月
    /// 星期
    pub week: Week,
    /// 法定节假日
    pub holiday: Option<LegalHoliday>, // 节假日
    /// 阳历节日
    pub festival: Option<SolarFestival>, // 阳历节日
}

/// 农历数据结构体
///
/// 包含农历日期、干支、生肖、节气、宜忌等信息
#[derive(Debug, Clone)]
pub struct LunarData {
    /// 农历年干支（60个周期）
    pub ganzhi: SixtyCycle, // 农历年干支（60个周期）
    /// 生肖
    pub zodiac: Zodiac, // 生肖
    /// 农历日
    pub day: LunarDay, // 农历日名称，该结构体内自带年、月
    /// 节气
    pub jieqi: Option<SolarTerm>, // 节气
    /// 适宜的事件列表（最多8个）
    pub day_recommends: heapless::Vec<Taboo, 8>, // 适宜的事件
    /// 避免的事件列表（最多8个）
    pub day_avoids: heapless::Vec<Taboo, 8>, // 避免的事件
    /// 农历节日
    pub festival: Option<LunarFestival>, // 农历节日
}

impl Default for LunarData {
    /// 提供农历数据的默认实现
    ///
    /// 返回一个包含默认值的农历数据实例
    /// 主要用于测试和初始化
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

/// 电池电量等级枚举
///
/// 表示设备的电池电量等级
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum BatteryLevel {
    /// 电量等级0 (最低)
    #[default]
    Level0 = 0,
    /// 电量等级1
    Level1,
    /// 电量等级2
    Level2,
    /// 电量等级3
    Level3,
    /// 电量等级4 (最高)
    Level4,
}

impl BatteryLevel {
    /// 根据电池电量百分比创建电池电量等级
    ///
    /// # 参数
    ///
    /// * `percent` - 电池电量百分比 (0-100)
    ///
    /// # 返回值
    ///
    /// * `BatteryLevel` - 对应的电池电量等级
    pub fn from_percent(percent: u8) -> Self {
        match percent {
            0..=20 => Self::Level0,
            21..=40 => Self::Level1,
            41..=60 => Self::Level2,
            61..=80 => Self::Level3,
            _ => Self::Level4,
        }
    }
}

/// 天气数据结构体
///
/// 包含传感器数据和天气预报信息
#[derive(Debug, Clone, Default)]
pub struct WeatherData {
    /// 温湿度传感器的数据 (温度, 湿度)
    pub sensor_data: Option<(f32, f32)>,
    /// 天气数据的位置及3天的预报数据
    pub daily_forecast: Option<(heapless::String<20>, [DailyWeather; 3])>,
}
