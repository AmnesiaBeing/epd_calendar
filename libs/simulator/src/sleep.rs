//! 模拟器睡眠管理
//!
//! 模拟器使用 Task 重启来模拟 Deep Sleep 行为：
//! - HTTP 服务器保持独立运行
//! - 模拟器逻辑在 Task 中运行，Deep Sleep 后重启

use embassy_time::Duration;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

use lxx_calendar_common::traits::platform::{RtcMemoryData, SleepManager, SleepMode, WakeupSource};

/// 模拟器睡眠管理器（可克隆）
#[derive(Clone)]
pub struct SimulatorSleepManager {
    rtc_memory: Arc<Mutex<RtcMemoryData>>,
    wakeup_notify: Arc<Notify>,
}

impl SimulatorSleepManager {
    pub fn new(rtc_memory: Arc<Mutex<RtcMemoryData>>, wakeup_notify: Arc<Notify>) -> Self {
        Self {
            rtc_memory,
            wakeup_notify,
        }
    }
}

impl SleepManager for SimulatorSleepManager {
    type Error = SimulatorSleepError;

    async fn sleep(
        &mut self,
        mode: SleepMode,
        duration: Duration,
    ) -> Result<WakeupSource, Self::Error> {
        match mode {
            SleepMode::LightSleep => {
                // Light Sleep: 等待指定时间后返回
                tokio::time::sleep(std::time::Duration::from_millis(duration.as_millis())).await;

                // 更新唤醒源
                {
                    let mut mem = self.rtc_memory.lock().await;
                    mem.wakeup_source = WakeupSource::RtcTimer;
                }

                Ok(WakeupSource::RtcTimer)
            }
            SleepMode::DeepSleep => {
                // Deep Sleep: 保存状态，等待，然后返回（Task 会重启）
                {
                    let mut mem = self.rtc_memory.lock().await;
                    mem.wakeup_source = WakeupSource::RtcTimer;
                    mem.last_update_time = get_timestamp();
                }

                // 等待（模拟 Deep Sleep）
                tokio::time::sleep(std::time::Duration::from_millis(duration.as_millis())).await;

                // 通知唤醒
                self.wakeup_notify.notify_one();

                // 返回后 Task 结束，会被外层循环重启
                Ok(WakeupSource::RtcTimer)
            }
        }
    }

    fn get_wakeup_source(&self) -> WakeupSource {
        // 从 RTC 内存读取唤醒源（使用 blocking_lock 用于同步上下文）
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

/// 模拟器睡眠错误
#[derive(Debug, Clone)]
pub enum SimulatorSleepError {
    InternalError,
}

impl core::fmt::Display for SimulatorSleepError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SimulatorSleepError")
    }
}

impl std::error::Error for SimulatorSleepError {}

/// 获取当前时间戳（秒）
fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
