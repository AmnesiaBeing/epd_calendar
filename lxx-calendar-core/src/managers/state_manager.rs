use lxx_calendar_common::*;

use crate::platform::*;

pub struct StateManager {
    event_channel: LxxChannelReceiver<'static, SystemEvent>,
    current_state: SystemMode,
}

impl StateManager {
    pub fn new(event_receiver: LxxChannelReceiver<'static, SystemEvent>) -> Self {
        Self {
            current_state: SystemMode::DeepSleep,
            event_channel: event_receiver,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing state manager");
        self.current_state = SystemMode::DeepSleep;
        Ok(())
    }

    pub async fn start(&mut self) -> SystemResult<()> {
        info!("Starting state manager");
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        info!("Stopping state manager");
        Ok(())
    }

    pub async fn handle_event(&mut self, event: SystemEvent) -> SystemResult<()> {
        info!("Handling event: {:?}", event);

        match event {
            SystemEvent::WakeupEvent(evt) => self.handle_wakeup_event(evt).await?,
            SystemEvent::UserEvent(evt) => self.handle_user_event(evt).await?,
            SystemEvent::TimeEvent(evt) => self.handle_time_event(evt).await?,
            SystemEvent::NetworkEvent(evt) => self.handle_network_event(evt).await?,
            SystemEvent::SystemStateEvent(evt) => self.handle_system_event(evt).await?,
            SystemEvent::PowerEvent(evt) => self.handle_power_event(evt).await?,
        }

        Ok(())
    }

    pub async fn get_current_state(&self) -> SystemResult<SystemMode> {
        Ok(self.current_state)
    }

    pub async fn transition_to(&mut self, mode: SystemMode) -> SystemResult<()> {
        info!("Transitioning from {:?} to {:?}", self.current_state, mode);

        if !self.can_transition(self.current_state, mode) {
            return Err(SystemError::HardwareError(HardwareError::InvalidParameter));
        }

        self.current_state = mode;
        info!("Transitioned to {:?}", mode);

        Ok(())
    }

    pub async fn wait_for_event(&mut self) -> SystemResult<SystemEvent> {
        Ok(self.event_channel.receive().await)
    }

    fn can_transition(&self, from: SystemMode, to: SystemMode) -> bool {
        match (from, to) {
            (SystemMode::DeepSleep, SystemMode::BleConnection) => true,
            (SystemMode::DeepSleep, SystemMode::NormalWork) => true,
            (SystemMode::NormalWork, SystemMode::BleConnection) => true,
            (SystemMode::NormalWork, SystemMode::DeepSleep) => true,
            (SystemMode::BleConnection, SystemMode::NormalWork) => true,
            (SystemMode::BleConnection, SystemMode::DeepSleep) => true,
            _ => false,
        }
    }

    async fn handle_wakeup_event(&mut self, event: WakeupEvent) -> SystemResult<()> {
        match event {
            WakeupEvent::WakeFromDeepSleep => {
                info!("Waking from deep sleep");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            WakeupEvent::WakeByLPU => {
                info!("Waking by LPU timer");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            WakeupEvent::WakeByButton => {
                info!("Waking by button");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            WakeupEvent::WakeByWDT => {
                warn!("Waking by watchdog");
                self.transition_to(SystemMode::NormalWork).await?;
            }
        }
        Ok(())
    }

    async fn handle_user_event(&mut self, event: UserEvent) -> SystemResult<()> {
        match event {
            UserEvent::ButtonShortPress => {
                info!("Button short press");
                if self.current_state == SystemMode::NormalWork {
                    self.transition_to(SystemMode::BleConnection).await?;
                }
            }
            UserEvent::ButtonLongPress => {
                info!("Button long press");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            UserEvent::BLEConfigReceived(_) => {
                info!("BLE config received");
            }
        }
        Ok(())
    }

    async fn handle_time_event(&mut self, event: TimeEvent) -> SystemResult<()> {
        match event {
            TimeEvent::MinuteTick => {
                debug!("Minute tick");
            }
            TimeEvent::HourChimeTrigger => {
                info!("Hour chime trigger");
            }
            TimeEvent::AlarmTrigger(_) => {
                info!("Alarm trigger");
            }
        }
        Ok(())
    }

    async fn handle_network_event(&mut self, event: NetworkEvent) -> SystemResult<()> {
        match event {
            NetworkEvent::NetworkSyncRequested => {
                info!("Network sync requested");
            }
            NetworkEvent::NetworkSyncComplete(_) => {
                info!("Network sync complete");
            }
            NetworkEvent::NetworkSyncFailed(_) => {
                error!("Network sync failed");
            }
        }
        Ok(())
    }

    async fn handle_system_event(&mut self, event: SystemStateEvent) -> SystemResult<()> {
        match event {
            SystemStateEvent::EnterDeepSleep => {
                info!("Entering deep sleep");
                self.transition_to(SystemMode::DeepSleep).await?;
            }
            SystemStateEvent::EnterBLEMode => {
                info!("Entering BLE mode");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            SystemStateEvent::EnterNormalMode => {
                info!("Entering normal mode");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            SystemStateEvent::ConfigChanged(_) => {
                info!("Config changed");
            }
            SystemStateEvent::LowPowerDetected => {
                warn!("Low power detected");
            }
            SystemStateEvent::OTATriggered => {
                info!("OTA triggered");
            }
            SystemStateEvent::OTAUpdateComplete => {
                info!("OTA update complete");
            }
        }
        Ok(())
    }

    async fn handle_power_event(&mut self, event: PowerEvent) -> SystemResult<()> {
        match event {
            PowerEvent::BatteryLevelChanged(_level) => {
                info!("Battery level changed");
            }
            PowerEvent::ChargingStateChanged(_charging) => {
                info!("Charging state changed");
            }
            PowerEvent::LowPowerModeChanged(_enabled) => {
                info!("Low power mode changed");
            }
        }
        Ok(())
    }
}
