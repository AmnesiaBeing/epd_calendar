use core::sync::atomic::Ordering;
use lxx_calendar_common::{
    info,
    traits::button::{ButtonDriver, ButtonEvent},
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SimulatorButton {
    callback: Arc<Mutex<Option<Box<dyn Fn(ButtonEvent) + Send + 'static>>>>,
    wakeup_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl SimulatorButton {
    pub fn new() -> Self {
        Self {
            callback: Arc::new(Mutex::new(None)),
            wakeup_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn set_wakeup_flag(&mut self, flag: Arc<std::sync::atomic::AtomicBool>) {
        self.wakeup_flag = flag;
    }

    pub fn simulate_press(&self, event: ButtonEvent) {
        // 触发已注册的回调
        if let Ok(guard) = self.callback.lock() {
            if let Some(ref cb) = *guard {
                cb(event);
            }
        }

        // 设置 wakeup flag，唤醒睡眠中的系统
        self.wakeup_flag.store(true, Ordering::SeqCst);
        info!("Button pressed, wakeup flag set");
    }
}

impl ButtonDriver for SimulatorButton {
    type Error = std::convert::Infallible;

    async fn register_press_callback<F>(&mut self, callback: F) -> Result<(), Self::Error>
    where
        F: Fn(ButtonEvent) + Send + 'static,
    {
        info!("Simulator button callback registered");
        if let Ok(mut guard) = self.callback.lock() {
            *guard = Some(Box::new(callback));
        }
        Ok(())
    }
}
