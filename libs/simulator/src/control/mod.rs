pub mod http_server;
pub mod types;

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use lxx_calendar_common::SystemEvent;
use lxx_calendar_common::traits::button::ButtonEvent;

use crate::ble::SimulatedBLE;
use crate::rtc::SimulatedRtc;
use crate::watchdog::SimulatedWdt;
use types::*;

pub struct SimulatorControl {
    rtc: Arc<Mutex<SimulatedRtc>>,
    watchdog: SimulatedWdt,
    button_callback: Arc<Mutex<Option<Box<dyn Fn(ButtonEvent) + Send + 'static>>>>,
    ble_config_callback: Arc<Mutex<Option<Box<dyn Fn(&[u8]) + Send + 'static>>>>,

    // 显示状态管理
    display_mode: Arc<Mutex<String>>,
    display_last_refresh: Arc<Mutex<Option<u64>>>,

    // 闹钟状态管理
    alarm_enabled: Arc<Mutex<bool>>,
    alarm_triggered: Arc<Mutex<bool>>,
    alarm_trigger_time: Arc<Mutex<Option<u64>>>,
}

impl SimulatorControl {
    pub fn new(rtc: SimulatedRtc, watchdog: SimulatedWdt) -> Self {
        Self {
            rtc: Arc::new(Mutex::new(rtc)),
            watchdog,
            button_callback: Arc::new(Mutex::new(None)),
            ble_config_callback: Arc::new(Mutex::new(None)),
            // 初始化显示状态
            display_mode: Arc::new(Mutex::new("normal".to_string())),
            display_last_refresh: Arc::new(Mutex::new(None)),
            // 初始化闹钟状态
            alarm_enabled: Arc::new(Mutex::new(false)),
            alarm_triggered: Arc::new(Mutex::new(false)),
            alarm_trigger_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new_with_shared_rtc(rtc: Arc<Mutex<SimulatedRtc>>, watchdog: SimulatedWdt) -> Self {
        Self {
            rtc,
            watchdog,
            button_callback: Arc::new(Mutex::new(None)),
            ble_config_callback: Arc::new(Mutex::new(None)),
            // 初始化显示状态
            display_mode: Arc::new(Mutex::new("normal".to_string())),
            display_last_refresh: Arc::new(Mutex::new(None)),
            // 初始化闹钟状态
            alarm_enabled: Arc::new(Mutex::new(false)),
            alarm_triggered: Arc::new(Mutex::new(false)),
            alarm_trigger_time: Arc::new(Mutex::new(None)),
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

    pub fn set_ble_config_callback(&self, callback: Box<dyn Fn(&[u8]) + Send + 'static>) {
        if let Ok(mut guard) = self.ble_config_callback.lock() {
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

    pub fn simulate_ble_config(&self, data: &[u8]) {
        if let Ok(guard) = self.ble_config_callback.lock() {
            if let Some(ref callback) = *guard {
                callback(data);
            }
        }
    }

    pub fn get_status(&self, ble: &SimulatedBLE) -> StatusResponse {
        let rtc = self.rtc.lock().unwrap();
        StatusResponse {
            rtc: RtcStatusResponse {
                timestamp: rtc.get_timestamp(),
                initialized: rtc.is_initialized(),
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
        let rtc = self.rtc.lock().unwrap();
        RtcStatusResponse {
            timestamp: rtc.get_timestamp(),
            initialized: rtc.is_initialized(),
        }
    }

    pub fn get_watchdog_status(&self) -> WatchdogStatusResponse {
        WatchdogStatusResponse {
            enabled: self.watchdog.is_enabled(),
            timeout_ms: self.watchdog.get_timeout_ms(),
        }
    }

    // ==================== 显示相关方法 ====================

    pub fn get_display_status(&self) -> DisplayStatusResponse {
        let mode = self.display_mode.lock().unwrap().clone();
        let last_refresh = *self.display_last_refresh.lock().unwrap();

        DisplayStatusResponse {
            initialized: true,
            mode,
            width: 800,
            height: 480,
            busy: false,
            last_refresh,
        }
    }

    pub fn refresh_display(&self, mode: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        *self.display_last_refresh.lock().unwrap() = Some(now);

        lxx_calendar_common::info!("Display refresh triggered: {}", mode);
    }

    pub fn set_display_mode(&self, mode: &str) {
        *self.display_mode.lock().unwrap() = mode.to_string();
        lxx_calendar_common::info!("Display mode set to: {}", mode);
    }

    // ==================== 闹钟相关方法 ====================

    pub fn get_alarm_status(&self) -> AlarmStatusResponse {
        AlarmStatusResponse {
            enabled: *self.alarm_enabled.lock().unwrap(),
            triggered: *self.alarm_triggered.lock().unwrap(),
            trigger_time: *self.alarm_trigger_time.lock().unwrap(),
            repeat: false,
        }
    }

    pub fn set_alarm(&self, trigger_time: Option<u64>) {
        *self.alarm_enabled.lock().unwrap() = true;
        *self.alarm_triggered.lock().unwrap() = false;
        *self.alarm_trigger_time.lock().unwrap() = trigger_time;

        lxx_calendar_common::info!("Alarm set for: {:?}", trigger_time);
    }

    pub fn cancel_alarm(&self) {
        *self.alarm_enabled.lock().unwrap() = false;
        *self.alarm_triggered.lock().unwrap() = false;
        *self.alarm_trigger_time.lock().unwrap() = None;

        lxx_calendar_common::info!("Alarm cancelled");
    }

    pub fn trigger_alarm(&self) {
        *self.alarm_triggered.lock().unwrap() = true;
        lxx_calendar_common::info!("Alarm triggered!");
    }
}
