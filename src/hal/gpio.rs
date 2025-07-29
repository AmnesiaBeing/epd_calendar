//! GPIO抽象接口，基于embedded-hal

use embedded_hal::digital::ErrorType;

/// 数字输出引脚 trait
pub trait OutputPin: ErrorType {
    /// 设置引脚为高电平
    fn set_high(&mut self) -> Result<(), Self::Error>;

    /// 设置引脚为低电平
    fn set_low(&mut self) -> Result<(), Self::Error>;
}

/// 数字输入引脚 trait
pub trait InputPin: ErrorType {
    /// 读取引脚电平
    fn is_high(&self) -> Result<bool, Self::Error>;

    /// 读取引脚电平
    fn is_low(&self) -> Result<bool, Self::Error> {
        self.is_high().map(|h| !h)
    }
}
