use alloc::boxed::Box;
use lxx_calendar_common::*;
use lxx_calendar_common::Watchdog;

use crate::managers::WatchdogManager;
use crate::services::{
    audio_service::AudioService, ble_service::BLEService, display_service::DisplayService,
    network_service::NetworkService, power_service::PowerManager, time_service::TimeService,
};

pub struct StateManager<'a, W: Watchdog> {
    event_channel: LxxChannelReceiver<'a, SystemEvent>,
    current_state: SystemMode,
    time_service: &'a mut TimeService,
    display_service: &'a mut DisplayService,
    network_service: &'a mut NetworkService,
    ble_service: &'a mut BLEService,
    power_manager: &'a mut PowerManager,
    audio_service: &'a mut AudioService,
    watchdog: WatchdogManager<W>,
}

impl<'a, W: Watchdog> StateManager<'a, W> {
    pub fn new(
        event_receiver: LxxChannelReceiver<'a, SystemEvent>,
        time_service: &'a mut TimeService,
        display_service: &'a mut DisplayService,
        network_service: &'a mut NetworkService,
        ble_service: &'a mut BLEService,
        power_manager: &'a mut PowerManager,
        audio_service: &'a mut AudioService,
        watchdog_device: W,
    ) -> Self {
        Self {
            current_state: SystemMode::DeepSleep,
            event_channel: event_receiver,
            time_service,
            display_service,
            network_service,
            ble_service,
            power_manager,
            audio_service,
            watchdog: WatchdogManager::new(watchdog_device),
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing state manager");
        self.watchdog.initialize().await?;
        self.current_state = SystemMode::DeepSleep;
        Ok(())
    }

    pub async fn start(&mut self) -> SystemResult<()> {
        info!("Starting state manager");
        Ok(())
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        info!("Stopping state manager");
        self.ble_service.stop().await?;
        let _ = self.network_service.disconnect().await;
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
        if !self.can_transition(self.current_state, mode) {
            warn!(
                "Invalid transition from {:?} to {:?}",
                self.current_state, mode
            );
            return Err(SystemError::HardwareError(HardwareError::InvalidParameter));
        }

        info!("Transitioning from {:?} to {:?}", self.current_state, mode);

        self.on_state_exit(self.current_state).await?;

        self.current_state = mode;

        Box::pin(self.on_state_enter(mode)).await?;

        info!("Transitioned to {:?}", mode);
        Ok(())
    }

    async fn on_state_enter(&mut self, mode: SystemMode) -> SystemResult<()> {
        match mode {
            SystemMode::DeepSleep => {
                info!("Entering deep sleep mode");
                self.stop().await?;
            }
            SystemMode::BleConnection => {
                info!("Entering BLE connection mode");
                self.ble_service.start().await?;
            }
            SystemMode::NormalWork => {
                info!("Entering normal work mode");
                self.execute_scheduled_tasks().await?;
            }
        }
        Ok(())
    }

    async fn on_state_exit(&mut self, mode: SystemMode) -> SystemResult<()> {
        match mode {
            SystemMode::DeepSleep => {
                info!("Exiting deep sleep mode");
            }
            SystemMode::BleConnection => {
                info!("Exiting BLE connection mode");
                self.ble_service.stop().await?;
            }
            SystemMode::NormalWork => {
                info!("Exiting normal work mode");
            }
        }
        Ok(())
    }

    async fn execute_scheduled_tasks(&mut self) -> SystemResult<()> {
        info!("Executing scheduled tasks");
        
        self.watchdog.start_task().await;

        let is_low_battery = self.power_manager.is_low_battery().await?;
        let schedule = self.time_service.calculate_wakeup_schedule().await?;

        if schedule.scheduled_tasks.network_sync && !is_low_battery {
            info!("Performing network sync");
            self.watchdog.feed();
            match self.network_service.sync().await {
                Ok(result) => {
                    if result.time_synced {
                        info!("Time synchronized successfully");
                    }
                    if result.weather_synced {
                        info!("Weather synchronized successfully");
                    }
                }
                Err(e) => {
                    warn!("Network sync failed: {:?}", e);
                }
            }
        }

        if schedule.scheduled_tasks.display_refresh {
            info!("Refreshing display");
            self.watchdog.feed();
            self.display_service.refresh().await?;
        }

        if schedule.scheduled_tasks.alarm_check {
            info!("Checking alarms");
            self.watchdog.feed();
        }

        let wakeup_time = self.time_service.calculate_next_wakeup_time().await?;
        info!("Next wakeup scheduled at: {:?}", wakeup_time);

        self.watchdog.end_task().await;
        
        self.transition_to(SystemMode::DeepSleep).await?;

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
