//! T-SPi 睡眠管理
//!
//! T-SPi 是半模拟平台：
//! - 使用真实 GPIO 和 SPI
//! - RTC 使用模拟器实现
//! - 睡眠通过等待实现

use embassy_time::Duration;
use lxx_calendar_common::traits::platform::{RtcMemoryData, SleepManager, SleepMode, WakeupSource};
use std::sync::Arc;
use tokio::sync::Mutex;

/// T-SPi 睡眠管理器
pub struct TspiSleepManager {
    rtc_memory: Arc<Mutex<RtcMemoryData>>,
}

impl TspiSleepManager {
    pub fn new(rtc_memory: Arc<Mutex<RtcMemoryData>>) -> Self {
        Self { rtc_memory }
    }
}

impl SleepManager for TspiSleepManager {
    type Error = TspiSleepError;

    async fn sleep(
        &mut self,
        mode: SleepMode,
        duration: Duration,
    ) -> Result<WakeupSource, Self::Error> {
        match mode {
            SleepMode::LightSleep => {
                // Light Sleep: 等待指定时间
                tokio::time::sleep(std::time::Duration::from_millis(duration.as_millis())).await;

                // 更新唤醒源
                {
                    let mut mem = self.rtc_memory.lock().await;
                    mem.wakeup_source = WakeupSource::RtcTimer;
                }

                Ok(WakeupSource::RtcTimer)
            }
            SleepMode::DeepSleep => {
                // Deep Sleep: 保存状态，等待，然后返回
                {
                    let mut mem = self.rtc_memory.lock().await;
                    mem.wakeup_source = WakeupSource::RtcTimer;
                    mem.last_update_time = get_timestamp();
                }

                // 等待（模拟 Deep Sleep）
                tokio::time::sleep(std::time::Duration::from_millis(duration.as_millis())).await;

                Ok(WakeupSource::RtcTimer)
            }
        }
    }

    fn get_wakeup_source(&self) -> WakeupSource {
        // 从 RTC 内存读取唤醒源
        match self.rtc_memory.try_lock() {
            Ok(mem) => {
                if mem.is_valid() {
                    mem.wakeup_source
                } else {
                    WakeupSource::PowerOn
                }
            }
            Err(_) => WakeupSource::PowerOn,
        }
    }

    fn save_rtc_memory(&mut self, data: RtcMemoryData) -> Result<(), Self::Error> {
        futures_executor::block_on(async {
            let mut mem = self.rtc_memory.lock().await;
            *mem = data;
        });
        Ok(())
    }

    fn load_rtc_memory(&self) -> Result<RtcMemoryData, Self::Error> {
        match self.rtc_memory.try_lock() {
            Ok(mem) => Ok(*mem),
            Err(_) => Ok(RtcMemoryData::new()),
        }
    }
}

/// T-SPi 睡眠错误
#[derive(Debug, Clone)]
pub enum TspiSleepError {
    InternalError,
}

impl core::fmt::Display for TspiSleepError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "TspiSleepError")
    }
}

impl std::error::Error for TspiSleepError {}

/// 获取当前时间戳（秒）
fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
