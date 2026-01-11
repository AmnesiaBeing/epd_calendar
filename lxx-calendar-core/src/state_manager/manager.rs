use lxx_calendar_common as lxxcc;
use lxxcc::{SystemEvent, SystemMode, SystemResult, SystemError};
use lxxcc::types::async_types::{AsyncRawMutex, ChannelReceiver};


pub struct StateManager<M: AsyncRawMutex + 'static> {
    event_channel: ChannelReceiver<'static, M, SystemEvent, 32>,
    current_state: SystemMode,
}

impl<M: AsyncRawMutex> StateManager<M> {
    pub fn new(event_receiver: ChannelReceiver<'static, M, SystemEvent, 32>) -> Self {
        Self {
            current_state: SystemMode::DeepSleep,
            event_channel: event_receiver,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        lxxcc::info!("Initializing state manager");
        self.current_state = SystemMode::DeepSleep;
        Ok(())
    }

    pub async fn start(&mut self) -> SystemResult<()> {
        lxxcc::info!("Starting state manager");
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        lxxcc::info!("Stopping state manager");
        Ok(())
    }

    pub async fn handle_event(&mut self, event: SystemEvent) -> SystemResult<()> {
        lxxcc::info!("Handling event: {:?}", event);

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
        lxxcc::info!("Transitioning from {:?} to {:?}", self.current_state, mode);

        if !self.can_transition(self.current_state, mode) {
            return Err(lxxcc::SystemError::HardwareError(lxxcc::HardwareError::InvalidParameter));
        }

        self.current_state = mode;
        lxxcc::info!("Transitioned to {:?}", mode);

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

    async fn handle_wakeup_event(&mut self, event: lxxcc::WakeupEvent) -> SystemResult<()> {
        match event {
            lxxcc::WakeupEvent::WakeFromDeepSleep => {
                lxxcc::info!("Waking from deep sleep");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            lxxcc::WakeupEvent::WakeByLPU => {
                lxxcc::info!("Waking by LPU timer");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            lxxcc::WakeupEvent::WakeByButton => {
                lxxcc::info!("Waking by button");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            lxxcc::WakeupEvent::WakeByWDT => {
                lxxcc::warn!("Waking by watchdog");
                self.transition_to(SystemMode::NormalWork).await?;
            }
        }
        Ok(())
    }

    async fn handle_user_event(&mut self, event: lxxcc::UserEvent) -> SystemResult<()> {
        match event {
            lxxcc::UserEvent::ButtonShortPress => {
                lxxcc::info!("Button short press");
                if self.current_state == SystemMode::NormalWork {
                    self.transition_to(SystemMode::BleConnection).await?;
                }
            }
            lxxcc::UserEvent::ButtonLongPress => {
                lxxcc::info!("Button long press");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            lxxcc::UserEvent::BLEConfigReceived(_) => {
                lxxcc::info!("BLE config received");
            }
        }
        Ok(())
    }

    async fn handle_time_event(&mut self, event: lxxcc::TimeEvent) -> SystemResult<()> {
        match event {
            lxxcc::TimeEvent::MinuteTick => {
                lxxcc::debug!("Minute tick");
            }
            lxxcc::TimeEvent::HourChimeTrigger => {
                lxxcc::info!("Hour chime trigger");
            }
            lxxcc::TimeEvent::AlarmTrigger(_) => {
                lxxcc::info!("Alarm trigger");
            }
        }
        Ok(())
    }

    async fn handle_network_event(&mut self, event: lxxcc::NetworkEvent) -> SystemResult<()> {
        match event {
            lxxcc::NetworkEvent::NetworkSyncRequested => {
                lxxcc::info!("Network sync requested");
            }
            lxxcc::NetworkEvent::NetworkSyncComplete(_) => {
                lxxcc::info!("Network sync complete");
            }
            lxxcc::NetworkEvent::NetworkSyncFailed(_) => {
                lxxcc::error!("Network sync failed");
            }
        }
        Ok(())
    }

    async fn handle_system_event(&mut self, event: lxxcc::SystemStateEvent) -> SystemResult<()> {
        match event {
            lxxcc::SystemStateEvent::EnterDeepSleep => {
                lxxcc::info!("Entering deep sleep");
                self.transition_to(SystemMode::DeepSleep).await?;
            }
            lxxcc::SystemStateEvent::EnterBLEMode => {
                lxxcc::info!("Entering BLE mode");
                self.transition_to(SystemMode::BleConnection).await?;
            }
            lxxcc::SystemStateEvent::EnterNormalMode => {
                lxxcc::info!("Entering normal mode");
                self.transition_to(SystemMode::NormalWork).await?;
            }
            lxxcc::SystemStateEvent::ConfigChanged(_) => {
                lxxcc::info!("Config changed");
            }
            lxxcc::SystemStateEvent::LowPowerDetected => {
                lxxcc::warn!("Low power detected");
            }
            lxxcc::SystemStateEvent::OTATriggered => {
                lxxcc::info!("OTA triggered");
            }
            lxxcc::SystemStateEvent::OTAUpdateComplete => {
                lxxcc::info!("OTA update complete");
            }
        }
        Ok(())
    }

    async fn handle_power_event(&mut self, event: lxxcc::PowerEvent) -> SystemResult<()> {
        match event {
            lxxcc::PowerEvent::BatteryLevelChanged(_level) => {
                lxxcc::info!("Battery level changed");
            }
            lxxcc::PowerEvent::ChargingStateChanged(_charging) => {
                lxxcc::info!("Charging state changed");
            }
            lxxcc::PowerEvent::LowPowerModeChanged(_enabled) => {
                lxxcc::info!("Low power mode changed");
            }
        }
        Ok(())
    }
}