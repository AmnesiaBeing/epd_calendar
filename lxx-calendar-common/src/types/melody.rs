//! 音符和旋律定义

/// 音符时值
#[derive(Clone, Copy, Debug)]
pub enum NoteDuration {
    Whole,     // 全音符 4拍
    Half,      // 二分音符 2拍
    Quarter,   // 四分音符 1拍
    Eighth,    // 八分音符 0.5拍
    Sixteenth, // 十六分音符 0.25拍
}

impl NoteDuration {
    pub const fn beats(&self) -> f32 {
        match self {
            NoteDuration::Whole => 4.0,
            NoteDuration::Half => 2.0,
            NoteDuration::Quarter => 1.0,
            NoteDuration::Eighth => 0.5,
            NoteDuration::Sixteenth => 0.25,
        }
    }

    pub const fn to_ms(&self, bpm: u32) -> u32 {
        let beat_ms = 60000 / bpm as u32;
        (self.beats() * beat_ms as f32) as u32
    }
}

/// 音符
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub freq: u32,
    pub duration: NoteDuration,
}

impl Note {
    pub const fn new(freq: u32, duration: NoteDuration) -> Self {
        Self { freq, duration }
    }

    pub const fn to_ms(&self, bpm: u32) -> u32 {
        self.duration.to_ms(bpm)
    }
}
