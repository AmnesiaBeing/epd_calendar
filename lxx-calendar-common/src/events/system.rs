use crate::types::{AlarmInfo, SyncResult, NetworkError, ConfigChange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {
    WakeupEvent(WakeupEvent),
    UserEvent(UserEvent),
    TimeEvent(TimeEvent),
    NetworkEvent(NetworkEvent),
    SystemStateEvent(SystemStateEvent),
    PowerEvent(PowerEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupEvent {
    WakeFromDeepSleep,
    WakeByLPU,
    WakeByButton,
    WakeByWDT,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    ButtonShortPress,
    ButtonLongPress,
    BLEConfigReceived(heapless::Vec<u8, 256>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeEvent {
    MinuteTick,
    HourChimeTrigger,
    AlarmTrigger(AlarmInfo),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    BatteryLevelChanged(u8),
    ChargingStateChanged(bool),
    LowPowerModeChanged(bool),
}
