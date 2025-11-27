use embassy_sync::once_lock::OnceLock;
use embassy_time::Instant;

// 全局状态
pub static SYSTEM_STATE: OnceLock<SystemState> = OnceLock::new();

#[derive(Debug)]
pub struct SystemState {
    pub last_sleep: Option<Instant>,
    pub last_wake: Instant,
    pub sleep_count: u32,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            last_sleep: None,
            last_wake: Instant::now(),
            sleep_count: 0,
        }
    }
}
