pub use sxtwl_rs::culture::Week;
pub use sxtwl_rs::festival::LunarFestival;
pub use sxtwl_rs::festival::SolarFestival;
pub use sxtwl_rs::lunar::LunarDay;
pub use sxtwl_rs::solar::SolarTerm;
pub use sxtwl_rs::solar::SolarTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlarmInfo {
    pub hour: u8,
    pub minute: u8,
    pub enabled: bool,
    pub repeat_days: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMode {
    DeepSleep,
    NormalWork,
    BleConnection,
}
