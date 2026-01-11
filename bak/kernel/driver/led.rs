// src/kernel/driver/actuators.rs
use crate::common::error::{AppError, Result};
use core::sync::atomic::{AtomicUsize, Ordering};
use embassy_executor::Spawner;

#[cfg(feature = "esp32c6")]
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    peripherals::Peripherals,
};

/// LED状态枚举（替换原Blink为快慢闪烁）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedState {
    /// LED关闭
    Off = 0,
    /// LED开启（常亮）
    On = 1,
    /// 快速闪烁（2Hz）
    FastBlink = 2,
    /// 慢速闪烁（0.5Hz）
    SlowBlink = 3,
}

impl LedState {
    pub fn from_usize(value: usize) -> Self {
        match value {
            0 => Self::Off,
            1 => Self::On,
            2 => Self::FastBlink,
            3 => Self::SlowBlink,
            _ => Self::Off,
        }
    }
}

/// 全局LED状态（用于任务间通信）
static LED_STATE: AtomicUsize = AtomicUsize::new(LedState::Off as usize);

/// 执行器驱动trait
pub trait LedDriver {
    /// 设置LED状态
    fn set_led_state(&mut self, state: LedState) -> Result<()>;
}

// ======================== 模拟驱动实现（Linux/模拟器） ========================
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub struct MockLedDriver {
    led_state: LedState,
}

#[cfg(any(feature = "simulator", feature = "tspi"))]
impl MockLedDriver {
    /// 创建并初始化模拟LED驱动
    pub fn new() -> Self {
        Self {
            led_state: LedState::Off,
        }
    }
}

#[cfg(any(feature = "simulator", feature = "tspi"))]
impl LedDriver for MockLedDriver {
    fn set_led_state(&mut self, state: LedState) -> Result<()> {
        self.led_state = state;
        LED_STATE.store(state as usize, Ordering::SeqCst);
        log::info!("Mock LED state set to: {:?}", state);
        Ok(())
    }
}

// ======================== ESP32C6驱动实现 ========================
#[cfg(feature = "esp32c6")]
pub struct EspLedDriver;

#[cfg(feature = "esp32c6")]
impl EspLedDriver {
    /// 初始化LED驱动（含LEDC配置+任务启动）
    pub fn new(peripherals: &Peripherals, spawner: &Spawner) -> Result<Self> {
        let led_pin = Output::new(
            unsafe { peripherals.GPIO6.clone_unchecked() },
            Level::Low,
            OutputConfig::default(),
        );

        // 6. 启动LED任务
        spawner.spawn(led_task(led_pin)).map_err(|e| {
            log::error!("启动LED任务失败: {:?}", e);
            AppError::LedError
        })?;

        Ok(Self)
    }
}

#[cfg(feature = "esp32c6")]
impl LedDriver for EspLedDriver {
    /// 设置LED状态（含亮度+闪烁控制）
    fn set_led_state(&mut self, state: LedState) -> Result<()> {
        LED_STATE.store(state as usize, Ordering::SeqCst);

        Ok(())
    }
}

// ======================== LED任务（embassy executor） ========================
#[cfg(feature = "esp32c6")]
#[embassy_executor::task]
async fn led_task(mut led_pin: Output<'static>) {
    use embassy_time::{Duration, Timer};

    loop {
        let state = LedState::from_usize(LED_STATE.load(Ordering::SeqCst));

        match state {
            LedState::Off => {
                led_pin.set_low();
                // 等待状态变化
                loop {
                    Timer::after(Duration::from_millis(10)).await;
                    let new_state = LedState::from_usize(LED_STATE.load(Ordering::SeqCst));
                    if new_state != LedState::Off {
                        break;
                    }
                }
            }

            LedState::On => {
                led_pin.set_high();
                // 等待状态变化
                loop {
                    Timer::after(Duration::from_millis(10)).await;
                    let new_state = LedState::from_usize(LED_STATE.load(Ordering::SeqCst));
                    if new_state != LedState::On {
                        break;
                    }
                }
            }

            LedState::FastBlink => {
                // 2Hz = 250ms高 + 250ms低
                loop {
                    let new_state = LedState::from_usize(LED_STATE.load(Ordering::SeqCst));
                    if new_state != LedState::FastBlink {
                        break;
                    }
                    led_pin.set_high();
                    Timer::after(Duration::from_millis(250)).await;

                    let new_state = LedState::from_usize(LED_STATE.load(Ordering::SeqCst));
                    if new_state != LedState::FastBlink {
                        break;
                    }
                    led_pin.set_low();
                    Timer::after(Duration::from_millis(250)).await;
                }
            }

            LedState::SlowBlink => {
                // 0.5Hz = 1000ms高 + 1000ms低
                loop {
                    let new_state = LedState::from_usize(LED_STATE.load(Ordering::SeqCst));
                    if new_state != LedState::SlowBlink {
                        break;
                    }
                    led_pin.set_high();
                    Timer::after(Duration::from_millis(1000)).await;

                    let new_state = LedState::from_usize(LED_STATE.load(Ordering::SeqCst));
                    if new_state != LedState::SlowBlink {
                        break;
                    }
                    led_pin.set_low();
                    Timer::after(Duration::from_millis(1000)).await;
                }
            }
        }
    }
}

// ======================== 默认驱动类型别名 ========================
#[cfg(feature = "esp32c6")]
pub type DefaultLedDriver = EspLedDriver;

#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type DefaultLedDriver = MockLedDriver;
