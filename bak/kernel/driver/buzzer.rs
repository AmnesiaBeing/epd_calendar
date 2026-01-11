// src/kernel/driver/buzzer.rs
//! 蜂鸣器驱动模块
//! 
//! 提供ESP32C平台的蜂鸣器驱动功能，使用LEDC外设控制无源蜂鸣器
//! 支持播放单个声音和预设音乐

use crate::common::error::{AppError, Result};

use embassy_time::{Duration, Timer};

#[cfg(feature = "esp32c6")]
use esp_hal::peripherals::Peripherals;

/// 蜂鸣器驱动trait
/// 
/// 定义蜂鸣器设备的通用操作接口
pub trait BuzzerDriver {
    /// 播放单个音调
    /// 
    /// # 参数
    /// - `frequency`: 音调频率，单位Hz
    /// - `duration`: 持续时间，单位毫秒
    async fn tone(&mut self, frequency: u32, duration: u32) -> Result<()>;
    
    /// 播放内置音乐
    /// 
    /// # 参数
    /// - `music_id`: 音乐序号，0 表示小星星，1 表示兰花草
    async fn play_music(&mut self, music_id: u8) -> Result<()>;
    
    /// 停止播放
    fn stop(&mut self) -> Result<()>;
}

// ======================== 模拟驱动实现（Linux/模拟器） ========================
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub struct MockBuzzerDriver {
    /// 模拟蜂鸣器状态
    is_playing: bool,
}

#[cfg(any(feature = "simulator", feature = "tspi"))]
impl MockBuzzerDriver {
    /// 创建模拟蜂鸣器驱动实例
    pub fn new() -> Result<Self> {
        Ok(Self {
            is_playing: false,
        })
    }
}

#[cfg(any(feature = "simulator", feature = "tspi"))]
impl BuzzerDriver for MockBuzzerDriver {
    async fn tone(&mut self, frequency: u32, duration: u32) -> Result<()> {
        self.is_playing = true;
        log::debug!("Mock buzzer playing tone: {} Hz for {} ms", frequency, duration);
        Timer::after(Duration::from_millis(duration as u64)).await;
        self.is_playing = false;
        Ok(())
    }
    
    async fn play_music(&mut self, music_id: u8) -> Result<()> {
        self.is_playing = true;
        log::debug!("Mock buzzer playing music with id: {}", music_id);
        // 模拟播放音乐，持续2秒
        Timer::after(Duration::from_secs(2)).await;
        self.is_playing = false;
        Ok(())
    }
    
    fn stop(&mut self) -> Result<()> {
        self.is_playing = false;
        log::debug!("Mock buzzer stopped");
        Ok(())
    }
}

// ======================== ESP32C6驱动实现 ========================
#[cfg(feature = "esp32c6")]
use esp_hal::{gpio::Output, ledc::{self, channel::Number as ChannelNumber, timer::Number as TimerNumber, timer::config::Config as TimerConfig, channel::config::Config as ChannelConfig, LowSpeed, timer::TimerIFace, channel::ChannelIFace}, time::Rate};

#[cfg(feature = "esp32c6")]
pub struct EspBuzzerDriver {
    /// LEDC通道
    channel: ledc::channel::Channel<'static, LowSpeed>,
    /// LEDC定时器
    timer: ledc::timer::Timer<'static, LowSpeed>,
}

#[cfg(feature = "esp32c6")]
impl EspBuzzerDriver {
    /// 创建ESP32C6蜂鸣器驱动实例
    /// 
    /// # 参数
    /// - `peripherals`: ESP32C6外设实例
    /// - `buzzer_pin`: 蜂鸣器连接的GPIO引脚
    pub fn new(peripherals: &mut Peripherals, buzzer_pin: Output<'static>) -> Result<Self> {
        // 配置LEDC
        let mut ledc = ledc::Ledc::new(unsafe { peripherals.LEDC.clone_unchecked() });
        
        // 设置全局慢速时钟源
        ledc.set_global_slow_clock(ledc::LSGlobalClkSource::APBClk);
        
        // 配置LEDC定时器
        let mut timer = ledc.timer::<LowSpeed>(TimerNumber::Timer0);
        timer.configure(TimerConfig {
            duty: ledc::timer::config::Duty::Duty13Bit,
            clock_source: ledc::timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(50),
        }).map_err(|_| AppError::ActuatorError)?;
        
        // 配置LEDC通道
        let mut channel = ledc.channel::<LowSpeed>(ChannelNumber::Channel0, buzzer_pin);
        channel.configure(ChannelConfig {
            timer: &timer,
            duty_pct: 0,
            drive_mode: esp_hal::gpio::DriveMode::PushPull,
        }).map_err(|_| AppError::ActuatorError)?;
        
        // 我们需要先将channel转换为'static生命周期，然后再移动timer
        let channel = unsafe { core::mem::transmute(channel) };
        let timer = unsafe { core::mem::transmute(timer) };
        
        Ok(Self {
            channel,
            timer,
        })
    }
}

#[cfg(feature = "esp32c6")]
impl BuzzerDriver for EspBuzzerDriver {
    async fn tone(&mut self, frequency: u32, duration: u32) -> Result<()> {
        // 重新配置定时器频率
        self.timer.configure(TimerConfig {
            duty: ledc::timer::config::Duty::Duty13Bit,
            clock_source: ledc::timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(frequency),
        }).map_err(|_| AppError::ActuatorError)?;
        
        // 设置50%占空比
        self.channel.set_duty(50).map_err(|_| AppError::ActuatorError)?;
        
        // 等待指定时长
        Timer::after(Duration::from_millis(duration as u64)).await;
        
        // 停止播放
        self.channel.set_duty(0).map_err(|_| AppError::ActuatorError)?;
        
        Ok(())
    }
    
    async fn play_music(&mut self, music_id: u8) -> Result<()> {
        match music_id {
            0 => self.play_twinkle().await,
            1 => self.play_orchid().await,
            _ => Err(AppError::ConfigError("Unsupported music id")),
        }
    }
    
    fn stop(&mut self) -> Result<()> {
        self.channel.set_duty(0).map_err(|_| AppError::ActuatorError)?;
        Ok(())
    }
}

#[cfg(feature = "esp32c6")]
impl EspBuzzerDriver {
    /// 播放小星星
    async fn play_twinkle(&mut self) -> Result<()> {
        // 小星星简谱：1 1 5 5 6 6 5 - 4 4 3 3 2 2 1 - 5 5 4 4 3 3 2 - 5 5 4 4 3 3 2 - 1 1 5 5 6 6 5 - 4 4 3 3 2 2 1
        // 音名对应：C C G G A A G - F F E E D D C - G G F F E E D - G G F F E E D - C C G G A A G - F F E E D D C
        // 频率表（单位Hz）
        const C4: u32 = 262;
        const D4: u32 = 294;
        const E4: u32 = 330;
        const F4: u32 = 349;
        const G4: u32 = 392;
        const A4: u32 = 440;
        
        // 小星星音符序列 (频率, 时长)
        let notes = [
            (C4, 250), (C4, 250), (G4, 250), (G4, 250), (A4, 250), (A4, 250), (G4, 500),
            (F4, 250), (F4, 250), (E4, 250), (E4, 250), (D4, 250), (D4, 250), (C4, 500),
            (G4, 250), (G4, 250), (F4, 250), (F4, 250), (E4, 250), (E4, 250), (D4, 500),
            (G4, 250), (G4, 250), (F4, 250), (F4, 250), (E4, 250), (E4, 250), (D4, 500),
            (C4, 250), (C4, 250), (G4, 250), (G4, 250), (A4, 250), (A4, 250), (G4, 500),
            (F4, 250), (F4, 250), (E4, 250), (E4, 250), (D4, 250), (D4, 250), (C4, 500),
        ];
        
        self.play_notes(&notes).await
    }
    
    /// 播放兰花草
    async fn play_orchid(&mut self) -> Result<()> {
        // 兰花草简谱片段：3 2 1 2 3 3 3 - 2 2 2 3 5 5 - 3 2 1 2 3 3 3 - 2 2 3 2 1
        // 音名对应：E D C D E E E - D D D E G G - E D C D E E E - D D E D C
        // 频率表（单位Hz）
        const C4: u32 = 262;
        const D4: u32 = 294;
        const E4: u32 = 330;
        const G4: u32 = 392;
        
        // 兰花草音符序列 (频率, 时长)
        let notes = [
            (E4, 250), (D4, 250), (C4, 250), (D4, 250), (E4, 250), (E4, 250), (E4, 500),
            (D4, 250), (D4, 250), (D4, 250), (E4, 250), (G4, 250), (G4, 500),
            (E4, 250), (D4, 250), (C4, 250), (D4, 250), (E4, 250), (E4, 250), (E4, 500),
            (D4, 250), (D4, 250), (E4, 250), (D4, 250), (C4, 500),
        ];
        
        self.play_notes(&notes).await
    }
    
    /// 播放音符序列
    async fn play_notes(&mut self, notes: &[(u32, u32)]) -> Result<()> {
        for (frequency, duration) in notes {
            self.tone(*frequency, *duration).await?;
            // 音符间短暂停顿
            Timer::after(Duration::from_millis(50)).await;
        }
        Ok(())
    }
}

// ======================== 默认驱动类型别名 ========================
#[cfg(feature = "esp32c6")]
pub type DefaultBuzzerDriver = EspBuzzerDriver;

#[cfg(any(feature = "simulator", feature = "tspi"))]
pub type DefaultBuzzerDriver = MockBuzzerDriver;