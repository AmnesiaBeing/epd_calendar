use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonEventType {
    ShortPress,
    LongPress,
    DoubleClick,
    TripleClick,
}

impl From<&ButtonEventType> for lxx_calendar_common::traits::button::ButtonEvent {
    fn from(event: &ButtonEventType) -> Self {
        match event {
            ButtonEventType::ShortPress => {
                lxx_calendar_common::traits::button::ButtonEvent::ShortPress
            }
            ButtonEventType::LongPress => {
                lxx_calendar_common::traits::button::ButtonEvent::LongPress
            }
            ButtonEventType::DoubleClick => {
                lxx_calendar_common::traits::button::ButtonEvent::DoubleClick
            }
            ButtonEventType::TripleClick => {
                lxx_calendar_common::traits::button::ButtonEvent::TripleClick
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ButtonRequest {
    pub event: ButtonEventType,
}

#[derive(Debug, Serialize)]
pub struct ButtonResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BleStatusResponse {
    pub connected: bool,
    pub advertising: bool,
    pub configured: bool,
}

#[derive(Debug, Serialize)]
pub struct BleConnectResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct BleConfigRequest {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct BleConfigResponse {
    pub success: bool,
    pub change: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RtcStatusResponse {
    pub timestamp: i64,
    pub initialized: bool,
}

#[derive(Debug, Serialize)]
pub struct WatchdogStatusResponse {
    pub enabled: bool,
    pub timeout_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub rtc: RtcStatusResponse,
    pub ble: BleStatusResponse,
    pub watchdog: WatchdogStatusResponse,
}

// ==================== 显示相关类型 ====================

#[derive(Debug, Serialize)]
pub struct DisplayStatusResponse {
    pub initialized: bool,
    pub mode: String,
    pub width: u16,
    pub height: u16,
    pub busy: bool,
    pub last_refresh: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct DisplayRefreshRequest {
    #[serde(default = "default_refresh_mode")]
    pub mode: String,
}

fn default_refresh_mode() -> String {
    "full".to_string()
}

#[derive(Debug, Serialize)]
pub struct DisplayRefreshResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct DisplayModeRequest {
    pub mode: String,
}

#[derive(Debug, Serialize)]
pub struct DisplayModeResponse {
    pub success: bool,
    pub message: String,
}

// ==================== 闹钟相关类型 ====================

#[derive(Debug, Serialize)]
pub struct AlarmStatusResponse {
    pub enabled: bool,
    pub triggered: bool,
    pub trigger_time: Option<u64>,
    pub repeat: bool,
}

#[derive(Debug, Deserialize)]
pub struct AlarmRequest {
    pub action: String,
    #[serde(default)]
    pub trigger_time: Option<String>,
    #[serde(default)]
    pub delay_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AlarmResponse {
    pub success: bool,
    pub message: String,
}
