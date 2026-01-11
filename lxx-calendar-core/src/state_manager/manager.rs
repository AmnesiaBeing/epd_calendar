use lxx_calendar_common as lxx_common;
use lxx_common::types::async_types::{LxxAsyncRawMutex, LxxChannelReceiver};
use lxx_common::{SystemEvent, SystemMode, SystemResult};

pub struct StateManager<M: LxxAsyncRawMutex + 'static> {
    event_channel: LxxChannelReceiver<'static, M, SystemEvent, 32>,
    current_state: SystemMode,
}

impl<M: LxxAsyncRawMutex> StateManager<M> {
    pub fn new(event_receiver: LxxChannelReceiver<'static, M, SystemEvent, 32>) -> Self {
        Self {
            current_state: SystemMode::DeepSleep,
            event_channel: event_receiver,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        lxx_common::info!("Initializing state manager");
        self.current_state = SystemMode::DeepSleep;
        Ok(())
    }

    pub async fn start(&mut self) -> SystemResult<()> {
        lxx_common::info!("Starting state manager");
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        lxx_common::info!("Stopping state manager");
        Ok(())
    }

    pub async fn handle_event(&mut self, event: SystemEvent) -> SystemResult<()> {
        lxx_common::info!("Handling event: {:?}", event);

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
        lxx_common::info!("Transitioning from {:?} to {:?}", self.current_state, mode);

        if !self.can_transition(self.current_state, mode) {
            return Err(lxx_common::SystemError::HardwareError(
                lxx_common::HardwareError::InvalidParameter,
            ));
        }

        self.current_state = mode;
        lxx_common::info!("Transitioned to {:?}", mode);

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

    async fn handle_wakeup_event(&mut self, event: lxx_common::WakeupEvent) -> SystemResult<()> {
        match event {
            lxx_common::WakeupEvent::WakeFromDeepSleep => {
                lxx_common::info!("Waking from deep sleep");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            lxx_common::WakeupEvent::WakeByLPU => {
                lxx_common::info!("Waking by LPU timer");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            lxx_common::WakeupEvent::WakeByButton => {
                lxx_common::info!("Waking by button");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            lxx_common::WakeupEvent::WakeByWDT => {
                lxx_common::warn!("Waking by watchdog");
                self.transition_to(SystemMode::NormalWork).await?;
            }
        }
        Ok(())
    }

    async fn handle_user_event(&mut self, event: lxx_common::UserEvent) -> SystemResult<()> {
        match event {
            lxx_common::UserEvent::ButtonShortPress => {
                lxx_common::info!("Button short press");
                if self.current_state == SystemMode::NormalWork {
                    self.transition_to(SystemMode::BleConnection).await?;
                }
            }
            lxx_common::UserEvent::ButtonLongPress => {
                lxx_common::info!("Button long press");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            lxx_common::UserEvent::BLEConfigReceived(_) => {
                lxx_common::info!("BLE config received");
            }
        }
        Ok(())
    }

    async fn handle_time_event(&mut self, event: lxx_common::TimeEvent) -> SystemResult<()> {
        match event {
            lxx_common::TimeEvent::MinuteTick => {
                lxx_common::debug!("Minute tick");
            }
            lxx_common::TimeEvent::HourChimeTrigger => {
                lxx_common::info!("Hour chime trigger");
            }
            lxx_common::TimeEvent::AlarmTrigger(_) => {
                lxx_common::info!("Alarm trigger");
            }
        }
        Ok(())
    }

    async fn handle_network_event(&mut self, event: lxx_common::NetworkEvent) -> SystemResult<()> {
        match event {
            lxx_common::NetworkEvent::NetworkSyncRequested => {
                lxx_common::info!("Network sync requested");
            }
            lxx_common::NetworkEvent::NetworkSyncComplete(_) => {
                lxx_common::info!("Network sync complete");
            }
            lxx_common::NetworkEvent::NetworkSyncFailed(_) => {
                lxx_common::error!("Network sync failed");
            }
        }
        Ok(())
    }

    async fn handle_system_event(
        &mut self,
        event: lxx_common::SystemStateEvent,
    ) -> SystemResult<()> {
        match event {
            lxx_common::SystemStateEvent::EnterDeepSleep => {
                lxx_common::info!("Entering deep sleep");
                self.transition_to(SystemMode::DeepSleep).await?;
            }
            lxx_common::SystemStateEvent::EnterBLEMode => {
                lxx_common::info!("Entering BLE mode");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            lxx_common::SystemStateEvent::EnterNormalMode => {
                lxx_common::info!("Entering normal mode");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            lxx_common::SystemStateEvent::ConfigChanged(_) => {
                lxx_common::info!("Config changed");
            }
            lxx_common::SystemStateEvent::LowPowerDetected => {
                lxx_common::warn!("Low power detected");
            }
            lxx_common::SystemStateEvent::OTATriggered => {
                lxx_common::info!("OTA triggered");
            }
            lxx_common::SystemStateEvent::OTAUpdateComplete => {
                lxx_common::info!("OTA update complete");
            }
        }
        Ok(())
    }

    async fn handle_power_event(&mut self, event: lxx_common::PowerEvent) -> SystemResult<()> {
        match event {
            lxx_common::PowerEvent::BatteryLevelChanged(_level) => {
                lxx_common::info!("Battery level changed");
            }
            lxx_common::PowerEvent::ChargingStateChanged(_charging) => {
                lxx_common::info!("Charging state changed");
            }
            lxx_common::PowerEvent::LowPowerModeChanged(_enabled) => {
                lxx_common::info!("Low power mode changed");
            }
        }
        Ok(())
    }
}
