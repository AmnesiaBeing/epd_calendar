// src/driver/time_source/linux.rs
//! Linux/Tspi/模拟器平台时间源驱动实现
//! 简化版：无实际时间存储，仅通过Instant获取当前时间，set_time仅打印日志

use std::time::Instant;

use jiff::Timestamp;

use crate::{
    common::error::{AppError, Result},
    driver::time_source::TimeDriver,
    platform::{Platform, SimulatorPlatform, TspiPlatform},
};

pub struct SimulatedRtc;

// 为模拟器平台实现TimeDriver
impl TimeDriver for SimulatedRtc {
    type P = SimulatorPlatform;

    /// 创建模拟RTC实例（仅适配接口，无实际初始化逻辑）
    fn create(peripherals: &mut <Self::P as Platform>::Peripherals) -> Result<Self> {
        log::info!("Initializing Simulator RTC time driver (simplified)");
        Ok(Self)
    }

    /// 获取当前时间：直接使用Instant::now()转换为Timestamp
    fn get_time(&self) -> Result<Timestamp> {
        // 获取Instant并转换为微秒级Timestamp
        let instant = Instant::now();
        let micros = instant.elapsed().as_micros() as i64;

        let timestamp = Timestamp::from_microsecond(micros).map_err(|_| AppError::TimeError)?;

        log::debug!("Current simulated RTC time: {}", timestamp);
        Ok(timestamp)
    }

    /// 设置时间：仅打印日志，无实际存储逻辑
    fn set_time(&mut self, new_time: Timestamp) -> Result<()> {
        let timestamp_us = new_time.as_microsecond();
        log::debug!("Set simulated RTC time (no-op): {} us", timestamp_us);
        Ok(())
    }
}
