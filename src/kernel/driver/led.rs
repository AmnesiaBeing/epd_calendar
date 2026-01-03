// src/kernel/driver/actuators.rs
#[cfg(feature = "embedded_esp")]
use esp_hal::ledc::LowSpeed;

/// 执行器模块
///
/// 本模块定义了LED和蜂鸣器等执行器的驱动接口和实现
use crate::common::error::Result;

#[derive(Debug, Clone, Copy)]
pub enum LedState {
    /// LED关闭
    Off,
    /// LED开启
    On,
    /// LED闪烁（预设频率）
    Blink,
}

/// 执行器驱动trait
///
/// 定义LED和蜂鸣器的通用控制接口
pub trait LedDriver {
    /// 设置LED状态
    ///
    /// # 参数
    /// - `state`: LED状态
    ///
    /// # 返回值
    /// - `Result<()>`: 设置结果
    fn set_led_state(&mut self, state: LedState) -> Result<()>;
}

/// 模拟执行器驱动实现
///
/// 用于Linux和模拟器环境，仅提供日志打印
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct MockLedDriver {
    /// LED状态
    led_state: LedState,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl MockLedDriver {
    /// 创建新的模拟LED驱动实例
    ///
    /// # 返回值
    /// - `MockLedDriver`: 新的模拟LED驱动实例
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
        log::info!("Mock LED state set to: {:?}", state);
        Ok(())
    }
}

/// ESP32C6执行器驱动实现
#[cfg(feature = "embedded_esp")]
pub struct EspLedDriver {
    /// LED通道（静态生命周期）
    led_channel: esp_hal::ledc::channel::Channel<'static, LowSpeed>,
}

#[cfg(feature = "embedded_esp")]
impl EspLedDriver {
    /// 创建新的ESP32C6执行器驱动实例
    ///
    /// # 参数
    /// - `peripherals`: ESP32硬件外设
    ///
    /// # 返回值
    /// - `Result<Self>`: 驱动实例或错误
    pub fn new(peripherals: &esp_hal::peripherals::Peripherals) -> Result<Self> {
        use crate::common::error::AppError;
        use esp_hal::{
            gpio::{DriveMode, Level, Output, OutputConfig},
            ledc::{
                LSGlobalClkSource, Ledc, LowSpeed,
                channel::{self, ChannelIFace},
                timer::{self, TimerIFace},
            },
            time::Rate,
        };

        // 初始化GPIO6为LED输出引脚
        let led_pin = Output::new(
            unsafe { peripherals.GPIO6.clone_unchecked() },
            Level::High,
            OutputConfig::default(),
        );

        // 初始化LEDC外设（静态化，避免生命周期问题）
        let mut ledc = Ledc::new(unsafe { peripherals.LEDC.clone_unchecked() });
        ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

        // 关键修复：将Timer泄漏为静态生命周期
        let led_timer = {
            let mut timer = ledc.timer::<LowSpeed>(timer::Number::Timer0);
            timer
                .configure(timer::config::Config {
                    duty: timer::config::Duty::Duty5Bit,
                    clock_source: timer::LSClockSource::APBClk,
                    frequency: Rate::from_hz(1), // 1Hz闪烁频率
                })
                .map_err(|e| {
                    log::error!("Failed to configure LEDC timer: {:?}", e);
                    AppError::LedError
                })?;
            // 泄漏Timer到堆上，获得'static生命周期
            alloc::boxed::Box::leak(alloc::boxed::Box::new(timer))
        };

        // 配置Channel（此时Timer是'static，满足引用要求）
        let mut led_channel = ledc.channel(channel::Number::Channel0, led_pin);
        led_channel
            .configure(channel::config::Config {
                timer: led_timer, // 静态引用，无生命周期问题
                duty_pct: 0,      // 初始关闭
                drive_mode: DriveMode::PushPull,
            })
            .map_err(|e| {
                log::error!("Failed to configure LEDC channel: {:?}", e);
                AppError::LedError
            })?;

        Ok(Self { led_channel })
    }
}

#[cfg(feature = "embedded_esp")]
impl LedDriver for EspLedDriver {
    fn set_led_state(&mut self, state: LedState) -> Result<()> {
        use esp_hal::ledc::channel::ChannelIFace;

        // Duty5Bit的最大值是31（2^5-1），需适配占空比
        let duty = match state {
            LedState::Off => 0,    // 0% 占空比（关闭）
            LedState::On => 31,    // 100% 占空比（全开）
            LedState::Blink => 15, // ~50% 占空比（闪烁）
        };

        self.led_channel.set_duty(duty).map_err(|e| {
            use crate::common::error::AppError;

            log::error!("Failed to set LED state: {:?}", e);
            AppError::LedError
        })?;

        log::debug!("LED state set to {:?} (duty: {})", state, duty);
        Ok(())
    }
}

/// 默认执行器驱动类型别名
///
/// 根据平台特性选择不同的执行器驱动实现
#[cfg(feature = "embedded_esp")]
pub type DefaultLedDriver = EspLedDriver;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultLedDriver = MockLedDriver;
