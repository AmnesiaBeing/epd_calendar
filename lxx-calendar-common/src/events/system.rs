use crate::types::{AlarmInfo, ConfigChange, NetworkError, SyncResult};

#[derive(Debug, PartialEq)]
pub enum SystemEvent {
    WakeupEvent(crate::events::WakeupEvent),
    UserEvent(crate::events::UserEvent),
    TimeEvent(crate::events::TimeEvent),
    NetworkEvent(crate::events::NetworkEvent),
    SystemStateEvent(crate::events::SystemStateEvent),
    PowerEvent(crate::events::PowerEvent),
    ConfigChanged(ConfigChange),
    BLEEvent(BLEEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupEvent {
    WakeByButton,
    WakeByWDT,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    ButtonDoubleClick,
    ButtonTripleClick,
    ButtonShortPress,
    ButtonLongPress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeEvent {
    MinuteTick,
    HourChimeTrigger,
    AlarmTrigger(AlarmInfo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkEvent {
    NetworkSyncRequested,
    NetworkSyncComplete(SyncResult),
    NetworkSyncFailed(NetworkError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemStateEvent {
    EnterBLEMode,
    EnterNormalMode,
    ConfigChanged(ConfigChange),
    LowPowerDetected,
    OTATriggered,
    OTAUpdateComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerEvent {
    ChargingStateChanged(bool),
    LowPowerModeChanged(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BLEEvent {
    WifiConfigReceived {
        ssid: heapless::String<32>,
        password: heapless::String<64>,
    },
    NetworkConfigReceived {
        location_id: heapless::String<16>,
        latitude: f64,
        longitude: f64,
        location_name: heapless::String<32>,
        sync_interval_minutes: u16,
        auto_sync: bool,
    },
    DisplayConfigReceived {
        refresh_interval_seconds: u16,
        low_power_refresh_enabled: bool,
    },
    TimeConfigReceived {
        timezone_offset: i32,
        hour_chime_enabled: bool,
    },
    PowerConfigReceived {
        low_power_mode_enabled: bool,
    },
    LogConfigReceived {
        log_level: crate::types::LogLevel,
        log_to_flash: bool,
    },
    CommandNetworkSync,
    CommandReboot,
    CommandFactoryReset,
    OTAStart,
    OTAData(heapless::Vec<u8, 256>),
    OTAComplete,
    OTACancel,
}
