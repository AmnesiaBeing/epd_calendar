use embassy_time::Duration;
use lxx_calendar_common::*;

pub struct AudioService<A: BuzzerDriver> {
    audio_device: Option<A>,
    initialized: bool,
}

impl<A: BuzzerDriver> AudioService<A> {
    pub fn new(audio_device: A) -> Self {
        Self {
            audio_device: Some(audio_device),
            initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing audio service");
        self.initialized = true;
        info!("Audio service initialized");
        Ok(())
    }

    pub async fn play_hour_chime(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        info!("Playing hour chime (4 short + 1 long)");

        if let Some(ref mut device) = self.audio_device {
            for _ in 0..4 {
                let _ = device.play_tone(440, 250);
                embassy_time::Timer::after(Duration::from_secs(1)).await;
            }
            embassy_time::Timer::after(Duration::from_secs(1)).await;
            let _ = device.play_tone(523, 1000);
        }

        info!("Hour chime completed");
        Ok(())
    }

    pub async fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> SystemResult<()> {
        info!("Playing tone: {}Hz for {}ms", frequency, duration_ms);

        if let Some(ref mut device) = self.audio_device {
            let _ = device.play_tone(frequency, duration_ms);
        }

        Ok(())
    }
}
