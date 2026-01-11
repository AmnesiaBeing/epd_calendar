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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LunarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub is_leap_month: bool,
    pub ganzhi_year: heapless::String<4>,
    pub ganzhi_month: heapless::String<4>,
    pub ganzhi_day: heapless::String<4>,
    pub zodiac: Zodiac,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zodiac {
    Rat,
    Ox,
    Tiger,
    Rabbit,
    Dragon,
    Snake,
    Horse,
    Goat,
    Monkey,
    Rooster,
    Dog,
    Pig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolarTerm {
    pub name: heapless::String<4>,
    pub date: u8,
    pub month: u8,
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
    LPU,
    WDT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMode {
    DeepSleep,
    NormalWork,
    BleConnection,
}
