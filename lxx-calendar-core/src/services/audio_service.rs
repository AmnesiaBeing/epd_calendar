use embassy_time::Duration;
use lxx_calendar_common::*;

pub struct AudioService<A: BuzzerDriver> {
    audio_device: Option<A>,
    initialized: bool,
    is_playing: bool,
    enabled: bool,
}

impl<A: BuzzerDriver> AudioService<A> {
    pub fn new(audio_device: A) -> Self {
        Self {
            audio_device: Some(audio_device),
            initialized: false,
            is_playing: false,
            enabled: true,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing audio service");
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
        self.is_playing = true;

        if let Some(ref mut device) = self.audio_device {
            for _ in 0..4 {
                let _ = device.play_tone(440, 250);
                embassy_time::Timer::after(Duration::from_millis(50)).await;
            }
            embassy_time::Timer::after(Duration::from_millis(200)).await;
            let _ = device.play_tone(523, 1000);
        }

        self.is_playing = false;
        info!("Hour chime completed");
        Ok(())
    }

    pub async fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> SystemResult<()> {
        if !self.enabled {
            return Ok(());
        }

        info!("Playing tone: {}Hz for {}ms", frequency, duration_ms);

        if let Some(ref mut device) = self.audio_device {
            let _ = device.play_tone(frequency, duration_ms);
        }

        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if self.is_playing {
            info!("Stopping audio playback");
            if let Some(ref mut device) = self.audio_device {
                let _ = device.stop();
            }
            self.is_playing = false;
        }

        Ok(())
    }

    pub async fn is_playing(&self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if let Some(ref device) = self.audio_device {
            return Ok(device.is_playing());
        }
        Ok(false)
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
}
