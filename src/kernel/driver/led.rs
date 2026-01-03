// src/kernel/driver/actuators.rs
use crate::common::error::{AppError, Result};
use core::sync::atomic::{AtomicUsize, Ordering};
use embassy_executor::Spawner;

#[cfg(feature = "embedded_esp")]
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
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct MockLedDriver {
    led_state: LedState,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl MockLedDriver {
    /// 创建并初始化模拟LED驱动
    pub fn new() -> Self {
        Self {
            led_state: LedState::Off,
        }
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl LedDriver for MockLedDriver {
    fn set_led_state(&mut self, state: LedState) -> Result<()> {
        self.led_state = state;
        LED_STATE.store(state as usize, Ordering::SeqCst);
        log::info!("Mock LED state set to: {:?}", state);
        Ok(())
    }
}

// ======================== ESP32C6驱动实现 ========================
#[cfg(feature = "embedded_esp")]
pub struct EspLedDriver;

#[cfg(feature = "embedded_esp")]
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

#[cfg(feature = "embedded_esp")]
impl LedDriver for EspLedDriver {
    /// 设置LED状态（含亮度+闪烁控制）
    fn set_led_state(&mut self, state: LedState) -> Result<()> {
        LED_STATE.store(state as usize, Ordering::SeqCst);

        Ok(())
    }
}

// ======================== LED任务（embassy executor） ========================
#[cfg(feature = "embedded_esp")]
#[embassy_executor::task]
async fn led_task(mut led_pin: Output<'static>) {
    use embassy_time::{Duration, Instant, Timer};
    // 记录LED当前电平状态（用于闪烁切换）
    let mut current_level = Level::Low;
    // 记录上次闪烁切换时间（用于不同频率控制）
    let mut last_toggle = Instant::now();

    // 主循环：每500ms判断一次状态
    loop {
        // 等待500ms（固定轮询间隔）
        Timer::after(Duration::from_millis(500)).await;

        // 加载当前LED状态
        let state = LedState::from_usize(LED_STATE.load(Ordering::SeqCst));

        match state {
            LedState::Off => {
                // 关闭LED：强制置低
                if current_level != Level::Low {
                    led_pin.set_level(Level::Low);
                    current_level = Level::Low;
                    log::debug!("LED状态: Off");
                }
            }

            LedState::On => {
                // 打开LED：强制置高
                if current_level != Level::High {
                    led_pin.set_level(Level::High);
                    current_level = Level::High;
                    log::debug!("LED状态: On");
                }
            }

            LedState::FastBlink => {
                // 快速闪烁：每500ms切换一次（轮询间隔正好500ms，直接切换）
                current_level = match current_level {
                    Level::Low => Level::High,
                    Level::High => Level::Low,
                };
                led_pin.set_level(current_level);
                log::debug!("FastBlink: LED {:?}", current_level);
            }

            LedState::SlowBlink => {
                // 慢速闪烁：每1000ms切换一次（需判断是否到时间）
                if last_toggle.elapsed() >= Duration::from_millis(1000) {
                    current_level = match current_level {
                        Level::Low => Level::High,
                        Level::High => Level::Low,
                    };
                    led_pin.set_level(current_level);
                    last_toggle = Instant::now();
                    log::debug!("SlowBlink: LED {:?}", current_level);
                }
            }
        }
    }
}

// ======================== 默认驱动类型别名 ========================
#[cfg(feature = "embedded_esp")]
pub type DefaultLedDriver = EspLedDriver;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultLedDriver = MockLedDriver;
