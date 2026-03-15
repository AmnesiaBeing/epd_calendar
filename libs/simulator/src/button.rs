use crate::rtc::SleepState;
use lxx_calendar_common::{
    info,
    traits::button::{ButtonDriver, ButtonEvent},
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SimulatorButton {
    callback: Arc<Mutex<Option<Box<dyn Fn(ButtonEvent) + Send + 'static>>>>,
    sleep_state: Option<SleepState>,
}

impl SimulatorButton {
    pub fn new() -> Self {
        Self {
            callback: Arc::new(Mutex::new(None)),
            sleep_state: None,
        }
    }

    pub fn set_sleep_state(&mut self, state: SleepState) {
        self.sleep_state = Some(state);
    }

    pub fn simulate_press(&self, event: ButtonEvent) {
        // 触发已注册的回调
        if let Ok(guard) = self.callback.lock() {
            if let Some(ref cb) = *guard {
                cb(event);
            }
        }

        // 唤醒睡眠中的系统
        if let Some(ref sleep_state) = self.sleep_state {
            sleep_state.request_wakeup();
        }
        info!("Button pressed, wakeup requested");
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
