// src/driver/time_source.rs

use jiff::Timestamp;

use crate::common::error::Result;

// 时间逻辑声明
// ESP32内部实际上使用两个u32的RTC寄存器存储时间，通过调用pub fn set_current_time_us(&self, current_time_us: u64)函数来写入寄存器
// ESP32可通过pub fn current_time_us(&self) -> u64来读取当前时间
// 模拟器内使用1个u64来存储时间
// 存储的时间类型均为Timestamp（u64），时区相关信息不在本代码做处理

// 关于SNTP和时间源的关系
// ESP32内，通过SNTP更新时间后（SNTP是外部调用的），会调用pub fn set_current_time_us(&self, current_time_us: u64)函数来写入寄存器
// 模拟器内，通过SNTP更新时间后，会调用SimulatedRtc::update_timestamp方法来更新时间戳

// 时间中断逻辑声明
// ESP32可以通过pub fn set_interrupt_handler(&mut self, handler: InterruptHandler)来设定中断时间
// 模拟器内，使用一个task来模拟中断时间

pub trait TimeSource {
    /// 获取当前时间（UTC时间戳）
    fn get_time(&self) -> Result<Timestamp>;

    /// 设置新时间
    fn set_time(&mut self, new_time: Timestamp) -> Result<()>;
}

// 默认时间源选择

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
mod linux;

#[cfg(feature = "embedded_esp")]
mod esp;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultTimeSource = linux::SimulatedRtc;

#[cfg(feature = "embedded_esp")]
pub type DefaultTimeSource = esp::RtcTimeSource;
