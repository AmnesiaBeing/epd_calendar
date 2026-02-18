use lxx_calendar_common::*;
use embassy_time::Duration;

pub struct AudioService {
    initialized: bool,
    is_playing: bool,
    current_melody: Option<Melody>,
    enabled: bool,
    volume: u8,
}

impl AudioService {
    pub fn new() -> Self {
        Self {
            initialized: false,
            is_playing: false,
            current_melody: None,
            enabled: true,
            volume: 80,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing audio service");
        
        // 预留LEDC PWM初始化接口
        // 实际实现需要在ESP32-C6上初始化LEDC外设
        
        self.enabled = true;
        self.initialized = true;
        
        info!("Audio service initialized");
        
        Ok(())
    }

    pub async fn play_hour_chime(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        
        if !self.enabled {
            info!("Audio disabled, skipping hour chime");
            return Ok(());
        }
        
        if self.is_playing {
            warn!("Audio already playing, skipping hour chime");
            return Ok(());
        }
        
        info!("Playing hour chime (4 short + 1 long)");
        
        // 4短 + 1长 整点报时
        // 短: 100ms, 长: 400ms
        // 间隔: 100ms
        self.play_tone(800, Duration::from_millis(100)).await?;
        self.play_tone(800, Duration::from_millis(100)).await?;
        self.play_tone(800, Duration::from_millis(100)).await?;
        self.play_tone(800, Duration::from_millis(100)).await?;
        
        embassy_time::Timer::after(Duration::from_millis(200)).await;
        
        self.play_tone(1000, Duration::from_millis(400)).await?;
        
        self.is_playing = false;
        self.current_melody = None;
        
        info!("Hour chime completed");
        
        Ok(())
    }

    pub async fn play_alarm(&mut self, melody: Melody) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        
        if !self.enabled {
            info!("Audio disabled, skipping alarm");
            return Ok(());
        }
        
        if self.is_playing {
            warn!("Audio already playing, stopping current playback");
            self.stop().await?;
        }
        
        info!("Playing alarm: {:?}", melody);
        
        self.is_playing = true;
        self.current_melody = Some(melody);
        
        match melody {
            Melody::Alarm1 => {
                self.play_xiaoxingxing().await?;
            }
            Melody::Alarm2 => {
                self.play_lanhua().await?;
            }
            Melody::HourChime => {
                return self.play_hour_chime().await;
            }
            Melody::Alarm3 | Melody::Custom => {
                self.play_default_alarm().await?;
            }
        }
        
        self.is_playing = false;
        self.current_melody = None;
        
        info!("Alarm completed");
        
        Ok(())
    }

    async fn play_xiaoxingxing(&mut self) -> SystemResult<()> {
        // 小星星旋律
        let notes = [
            (523, 250), (523, 250), (784, 250), (784, 250),
            (880, 250), (880, 250), (784, 500),
            (698, 250), (698, 250), (659, 250), (659, 250),
            (587, 250), (587, 250), (523, 500),
        ];
        
        for (freq, dur) in notes.iter() {
            self.play_tone(*freq, Duration::from_millis(*dur)).await?;
            embassy_time::Timer::after(Duration::from_millis(50)).await;
        }
        
        Ok(())
    }

    async fn play_lanhua(&mut self) -> SystemResult<()> {
        // 兰花草旋律
        let notes = [
            (356, 250), (395, 250), (395, 125), (356, 125), (316, 250),
            (296, 250), (296, 125), (296, 125), (316, 250), (356, 250),
            (356, 125), (395, 125), (395, 250), (316, 250), (296, 500),
        ];
        
        for (freq, dur) in notes.iter() {
            self.play_tone(*freq, Duration::from_millis(*dur)).await?;
            embassy_time::Timer::after(Duration::from_millis(50)).await;
        }
        
        Ok(())
    }

    async fn play_default_alarm(&mut self) -> SystemResult<()> {
        // 默认报警音
        for _ in 0..3 {
            self.play_tone(1000, Duration::from_millis(200)).await?;
            embassy_time::Timer::after(Duration::from_millis(100)).await;
        }
        
        Ok(())
    }

    async fn play_tone(&mut self, frequency: u32, duration: Duration) -> SystemResult<()> {
        if !self.enabled {
            return Ok(());
        }
        
        info!("Playing tone: {}Hz for {}ms", frequency, duration.as_millis());
        
        // 预留PWM输出接口
        // 实际实现需要在ESP32-C6上配置LEDC PWM输出
        
        embassy_time::Timer::after(duration).await;
        
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        
        if self.is_playing {
            info!("Stopping audio playback");
            
            // 预留停止PWM输出接口
            // 实际实现需要关闭LEDC PWM
            
            self.is_playing = false;
            self.current_melody = None;
        }
        
        Ok(())
    }

    pub async fn is_playing(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.is_playing)
    }

    pub async fn set_enabled(&mut self, enabled: bool) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        
        self.enabled = enabled;
        
        if !enabled && self.is_playing {
            self.stop().await?;
        }
        
        info!("Audio enabled: {}", enabled);
        
        Ok(())
    }

    pub async fn set_volume(&mut self, volume: u8) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        
        if volume > 100 {
            return Err(SystemError::HardwareError(HardwareError::InvalidParameter));
        }
        
        self.volume = volume;
        
        info!("Audio volume set to {}%", volume);
        
        Ok(())
    }

    pub async fn get_volume(&self) -> SystemResult<u8> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        Ok(self.volume)
    }
}
