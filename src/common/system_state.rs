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

impl SystemState {
    pub fn init() -> Result<(), &'static str> {
        SYSTEM_STATE
            .set(SystemState {
                last_sleep: None,
                last_wake: Instant::now(),
                sleep_count: 0,
            })
            .map_err(|_| "SystemState already initialized")
    }

    pub fn get() -> Option<&'static SystemState> {
        SYSTEM_STATE.get()
    }

    pub fn mark_sleep(&self) {
        // 注意：这里需要内部可变性，或者使用Mutex
        // 简化版本：记录睡眠时间到外部存储
    }

    pub fn mark_wake(&self) {
        // 记录唤醒时间
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
