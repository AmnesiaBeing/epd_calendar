pub mod config;
pub mod display;
pub mod error;
pub mod layout;
pub mod melody;
pub mod time;
pub mod weather;

pub use config::*;
pub use display::*;
pub use error::*;
pub use layout::*;
pub use melody::*;
pub use time::*;
pub use weather::*;

/// 农历日期信息
#[derive(Debug, Clone)]
pub struct LunarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub zodiac: &'static str,
    pub ganzhi_year: &'static str,
}
