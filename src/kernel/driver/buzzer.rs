// src/kernel/driver/buzzer.rs
//! 蜂鸣器驱动模块
//! 
//! 提供ESP32C平台的蜂鸣器驱动功能，使用LEDC外设控制无源蜂鸣器
//! 支持播放单个声音和预设音乐

use crate::common::error::{AppError, Result};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

#[cfg(feature = "embedded_esp")]
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
    /// - `music`: 音乐名称，支持 "twinkle" (小星星) 和 "orchid" (兰花草)
    async fn play_music(&mut self, music: &str) -> Result<()>;
    
    /// 停止播放
    fn stop(&mut self) -> Result<()>;
}

// ======================== 模拟驱动实现（Linux/模拟器） ========================
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct MockBuzzerDriver {
    /// 模拟蜂鸣器状态
    is_playing: bool,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl MockBuzzerDriver {
    /// 创建模拟蜂鸣器驱动实例
    pub fn new() -> Result<Self> {
        Ok(Self {
            is_playing: false,
        })
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl BuzzerDriver for MockBuzzerDriver {
    async fn tone(&mut self, frequency: u32, duration: u32) -> Result<()> {
        self.is_playing = true;
        log::debug!("Mock buzzer playing tone: {} Hz for {} ms", frequency, duration);
        Timer::after(Duration::from_millis(duration as u64)).await;
        self.is_playing = false;
        Ok(())
    }
    
    async fn play_music(&mut self, music: &str) -> Result<()> {
        self.is_playing = true;
        log::debug!("Mock buzzer playing music: {}", music);
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
#[cfg(feature = "embedded_esp")]
use esp_hal::{gpio::Output, ledc::{self, channel::ChannelRef, timer::TimerRef}};

#[cfg(feature = "embedded_esp")]
pub struct EspBuzzerDriver {
    /// LEDC通道
    channel: ChannelRef<'static, ledc::channel::CH0>,
    /// LEDC定时器
    timer: TimerRef<'static, ledc::timer::TIMER0>,
}

#[cfg(feature = "embedded_esp")]
impl EspBuzzerDriver {
    /// 创建ESP32C6蜂鸣器驱动实例
    /// 
    /// # 参数
    /// - `peripherals`: ESP32C6外设实例
    /// - `buzzer_pin`: 蜂鸣器连接的GPIO引脚
    pub fn new(peripherals: &mut Peripherals, buzzer_pin: Output<'static>) -> Result<Self> {
        // 配置LEDC
        let mut ledc = ledc::Ledc::new(peripherals.LEDC);
        
        // 配置LEDC定时器
        let timer = ledc.get_timer(ledc::timer::TIMER0).unwrap();
        timer.set_frequency(50.kHz())?;
        timer.set_resolution(ledc::Resolution::Bits13)?;
        timer.enable()?;
        
        // 配置LEDC通道
        let channel = ledc.get_channel(ledc::channel::CH0).unwrap();
        channel.attach_pin(buzzer_pin)?;
        channel.attach_timer(timer)?;
        
        // 设置默认占空比为0（关闭）
        channel.set_duty(0)?;
        channel.enable()?;
        
        Ok(Self {
            channel,
            timer,
        })
    }
}

#[cfg(feature = "embedded_esp")]
impl BuzzerDriver for EspBuzzerDriver {
    async fn tone(&mut self, frequency: u32, duration: u32) -> Result<()> {
        // 设置频率
        self.timer.set_frequency(frequency.Hz())?;
        
        // 设置50%占空比
        let max_duty = self.channel.get_max_duty();
        self.channel.set_duty(max_duty / 2)?;
        
        // 等待指定时长
        Timer::after(Duration::from_millis(duration as u64)).await;
        
        // 停止播放
        self.channel.set_duty(0)?;
        
        Ok(())
    }
    
    async fn play_music(&mut self, music: &str) -> Result<()> {
        match music.to_lowercase().as_str() {
            "twinkle" => self.play_twinkle().await,
            "orchid" => self.play_orchid().await,
            _ => Err(AppError::InvalidInput("Unsupported music".to_string())),
        }
    }
    
    fn stop(&mut self) -> Result<()> {
        self.channel.set_duty(0)?;
        Ok(())
    }
}

#[cfg(feature = "embedded_esp")]
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
#[cfg(feature = "embedded_esp")]
pub type DefaultBuzzerDriver = EspBuzzerDriver;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultBuzzerDriver = MockBuzzerDriver;