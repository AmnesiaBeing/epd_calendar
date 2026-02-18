use sxtwl_rs::culture::ZODIAC_NAMES;
use sxtwl_rs::sixtycycle::SIXTY_CYCLE_NAMES;
use sxtwl_rs::solar::SOLAR_TERM_NAMES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub weekday: u8,
    pub timezone_offset: i32,
}

impl DateTime {
    pub fn new(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        weekday: u8,
        timezone_offset: i32,
    ) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            weekday,
            timezone_offset,
        }
    }
}

pub fn get_weekday_name(weekday: u8) -> &'static str {
    const WEEK_NAMES: [&str; 7] = ["日", "一", "二", "三", "四", "五", "六"];
    WEEK_NAMES[weekday as usize % 7]
}

pub fn get_month_name(month: u8) -> &'static str {
    const MONTH_NAMES: [&str; 12] = [
        "一月",
        "二月",
        "三月",
        "四月",
        "五月",
        "六月",
        "七月",
        "八月",
        "九月",
        "十月",
        "十一月",
        "十二月",
    ];
    MONTH_NAMES[(month as usize - 1) % 12]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LunarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub is_leap: bool,
    pub zodiac: &'static str,
    pub ganzhi_year: &'static str,
    pub ganzhi_month: &'static str,
    pub ganzhi_day: &'static str,
}

impl LunarDate {
    pub fn from_sxtwl(
        year: u16,
        month: u8,
        day: u8,
        is_leap: bool,
        zodiac_idx: usize,
        ganzhi_year_idx: usize,
        ganzhi_month_idx: usize,
        ganzhi_day_idx: usize,
    ) -> Self {
        Self {
            year,
            month,
            day,
            is_leap,
            zodiac: ZODIAC_NAMES[zodiac_idx % 12],
            ganzhi_year: SIXTY_CYCLE_NAMES[ganzhi_year_idx % 60],
            ganzhi_month: SIXTY_CYCLE_NAMES[ganzhi_month_idx % 60],
            ganzhi_day: SIXTY_CYCLE_NAMES[ganzhi_day_idx % 60],
        }
    }

    pub fn get_month_name(&self) -> &'static str {
        const LUNAR_MONTH_NAMES: [&str; 12] = [
            "正月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "冬月",
            "腊月",
        ];
        LUNAR_MONTH_NAMES[(self.month as usize - 1) % 12]
    }

    pub fn get_day_name(&self) -> &'static str {
        const LUNAR_DAY_NAMES: [&str; 30] = [
            "初一", "初二", "初三", "初四", "初五", "初六", "初七", "初八", "初九", "初十", "十一",
            "十二", "十三", "十四", "十五", "十六", "十七", "十八", "十九", "二十", "廿一", "廿二",
            "廿三", "廿四", "廿五", "廿六", "廿七", "廿八", "廿九", "三十",
        ];
        LUNAR_DAY_NAMES[(self.day as usize - 1) % 30]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolarTerm {
    pub name: &'static str,
    pub date: u8,
    pub month: u8,
}

impl SolarTerm {
    pub fn from_index(index: usize, date: u8, month: u8) -> Option<Self> {
        if index == 0 || index > 24 {
            return None;
        }
        Some(Self {
            name: SOLAR_TERM_NAMES[index - 1],
            date,
            month,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holiday {
    NewYear,
    SpringFestival,
    Qingming,
    LaborDay,
    DragonBoat,
    MidAutumn,
    NationalDay,
}

impl Holiday {
    pub fn get_name(&self) -> &'static str {
        match self {
            Holiday::NewYear => "元旦",
            Holiday::SpringFestival => "春节",
            Holiday::Qingming => "清明节",
            Holiday::LaborDay => "劳动节",
            Holiday::DragonBoat => "端午节",
            Holiday::MidAutumn => "中秋节",
            Holiday::NationalDay => "国庆节",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlarmInfo {
    pub hour: u8,
    pub minute: u8,
    pub enabled: bool,
    pub repeat_days: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WakeupSchedule {
    pub next_wakeup_time: i64,
    pub wakeup_reason: WakeupReason,
    pub scheduled_tasks: ScheduledTasks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledTasks {
    pub display_refresh: bool,
    pub network_sync: bool,
    pub alarm_check: bool,
    pub reserved: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupReason {
    Timer,
    Button,
    WDT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMode {
    DeepSleep,
    NormalWork,
    BleConnection,
}
