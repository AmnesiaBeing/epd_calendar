pub mod http_server;
pub mod types;

use std::sync::{Arc, Mutex};

use lxx_calendar_common::SystemEvent;
use lxx_calendar_common::traits::button::ButtonEvent;

use crate::ble::SimulatedBLE;
use crate::rtc::SimulatedRtc;
use crate::watchdog::SimulatedWdt;
use types::*;

pub struct SimulatorControl {
    rtc: SimulatedRtc,
    watchdog: SimulatedWdt,
    button_callback: Arc<Mutex<Option<Box<dyn Fn(ButtonEvent) + Send + 'static>>>>,
}

impl SimulatorControl {
    pub fn new(rtc: SimulatedRtc, watchdog: SimulatedWdt) -> Self {
        Self {
            rtc,
            watchdog,
            button_callback: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new_dummy() -> Self {
        Self::new(SimulatedRtc::new(), SimulatedWdt::new(30000))
    }

    pub fn set_button_callback(&self, callback: Box<dyn Fn(ButtonEvent) + Send + 'static>) {
        if let Ok(mut guard) = self.button_callback.lock() {
            *guard = Some(callback);
        }
    }

    pub fn simulate_button_press(&self, event: ButtonEventType) {
        let btn_event = ButtonEvent::from(&event);

        if let Ok(guard) = self.button_callback.lock() {
            if let Some(ref callback) = *guard {
                callback(btn_event);
            }
        }
    }

    pub fn get_status(&self, ble: &SimulatedBLE) -> StatusResponse {
        StatusResponse {
            rtc: RtcStatusResponse {
                timestamp: self.rtc.get_timestamp(),
                initialized: self.rtc.is_initialized(),
            },
            ble: BleStatusResponse {
                connected: ble.is_connected(),
                advertising: ble.is_advertising(),
                configured: ble.is_configured(),
            },
            watchdog: WatchdogStatusResponse {
                enabled: self.watchdog.is_enabled(),
                timeout_ms: self.watchdog.get_timeout_ms(),
            },
        }
    }

    pub fn get_ble_status(&self, ble: &SimulatedBLE) -> BleStatusResponse {
        BleStatusResponse {
            connected: ble.is_connected(),
            advertising: ble.is_advertising(),
            configured: ble.is_configured(),
        }
    }

    pub fn get_rtc_status(&self) -> RtcStatusResponse {
        RtcStatusResponse {
            timestamp: self.rtc.get_timestamp(),
            initialized: self.rtc.is_initialized(),
        }
    }

    pub fn get_watchdog_status(&self) -> WatchdogStatusResponse {
        WatchdogStatusResponse {
            enabled: self.watchdog.is_enabled(),
            timeout_ms: self.watchdog.get_timeout_ms(),
        }
    }
}
