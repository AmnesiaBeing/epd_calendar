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
