use crate::types::{AlarmInfo, ConfigChange, NetworkError, SyncResult};

#[derive(Debug, PartialEq, Eq)]
pub enum SystemEvent {
    WakeupEvent(crate::events::WakeupEvent),
    UserEvent(crate::events::UserEvent),
    TimeEvent(crate::events::TimeEvent),
    NetworkEvent(crate::events::NetworkEvent),
    SystemStateEvent(crate::events::SystemStateEvent),
    PowerEvent(crate::events::PowerEvent),
    ConfigChanged(ConfigChange),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupEvent {
    WakeFromDeepSleep,
    WakeByButton,
    WakeByWDT,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    ButtonShortPress,
    ButtonLongPress,
    BLEConfigReceived(heapless::Vec<u8, 64>),
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
    EnterDeepSleep,
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
