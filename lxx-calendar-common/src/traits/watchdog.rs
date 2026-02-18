//! 通用看门狗 trait

use core::convert::Infallible;

/// 通用看门狗 trait
///
/// 用于抽象不同平台的看门狗实现，包括：
/// - 硬件看门狗（如 ESP32 的 WDT）
/// - 软件模拟看门狗
///
/// 如果平台使用 embedded-hal 或其他标准接口，则不需要实现此 trait
pub trait Watchdog {
    /// 错误类型
    type Error;

    /// 喂狗 - 重置看门狗计时器
    ///
    /// 调用此方法可以阻止看门狗超时复位系统
    fn feed(&mut self) -> Result<(), Self::Error>;

    /// 启用看门狗
    fn enable(&mut self) -> Result<(), Self::Error>;

    /// 禁用看门狗
    fn disable(&mut self) -> Result<(), Self::Error>;

    /// 获取当前超时时间（毫秒）
    fn get_timeout(&self) -> Result<u32, Self::Error>;

    /// 设置超时时间（毫秒）
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<(), Self::Error>;
}

/// 允许空实现的 Watchdog（用于不需要看门狗的平台）
impl Watchdog for () {
    type Error = Infallible;

    fn feed(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn get_timeout(&self) -> Result<u32, Self::Error> {
        Ok(0)
    }

    fn set_timeout(&mut self, _timeout_ms: u32) -> Result<(), Self::Error> {
        Ok(())
    }
}
