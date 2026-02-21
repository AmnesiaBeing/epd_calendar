use alloc::boxed::Box;
use embassy_executor::Spawner;
use embassy_time::Duration;
use lxx_calendar_common::*;

use crate::managers::WatchdogManager;
use crate::services::{
    audio_service::AudioService,
    ble_service::BLEService,
    display_manager::DisplayManager,
    display_service::DisplayService,
    network_sync_service::NetworkSyncService,
    power_service::PowerManager,
    quote_service::QuoteService,
    time_service::{TimeService, WakeupSource},
};

pub struct StateManager<'a, P: PlatformTrait> {
    event_channel: LxxChannelReceiver<'a, SystemEvent>,
    current_state: SystemMode,
    time_service: &'a mut TimeService<P::RtcDevice>,
    display_service: &'a mut DisplayService,
    quote_service: &'a mut QuoteService,
    ble_service: &'a mut BLEService,
    power_manager: &'a mut PowerManager,
    audio_service: &'a mut AudioService<P::AudioDevice>,
    network_service: &'a mut NetworkSyncService<P::RtcDevice>,
    watchdog: WatchdogManager<P::WatchdogDevice>,
    config: Option<&'a lxx_calendar_common::SystemConfig>,
    last_chime_hour: Option<u8>,
    last_sync_time: Option<u64>,
}

impl<'a, P: PlatformTrait> StateManager<'a, P> {
    pub fn new(
        event_receiver: LxxChannelReceiver<'a, SystemEvent>,
        time_service: &'a mut TimeService<P::RtcDevice>,
        display_service: &'a mut DisplayService,
        quote_service: &'a mut QuoteService,
        ble_service: &'a mut BLEService,
        power_manager: &'a mut PowerManager,
        audio_service: &'a mut AudioService<P::AudioDevice>,
        network_service: &'a mut NetworkSyncService<P::RtcDevice>,
        watchdog_device: P::WatchdogDevice,
    ) -> Self {
        Self {
            current_state: SystemMode::DeepSleep,
            event_channel: event_receiver,
            time_service,
            display_service,
            quote_service,
            ble_service,
            power_manager,
            audio_service,
            network_service,
            watchdog: WatchdogManager::new(watchdog_device),
            config: None,
            last_chime_hour: None,
            last_sync_time: None,
        }
    }

    pub fn with_config(&mut self, config: &'a lxx_calendar_common::SystemConfig) {
        self.config = Some(config);
        self.last_chime_hour = None;
        self.last_sync_time = None;
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing state manager");
        self.watchdog.initialize().await?;
        self.current_state = SystemMode::DeepSleep;
        Ok(())
    }

    pub fn feed_watchdog(&mut self) {
        self.watchdog.feed();
    }

    pub async fn stop(&mut self) -> SystemResult<()> {
        info!("Stopping state manager");
        self.ble_service.stop().await?;
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
            SystemEvent::ConfigChanged(change) => self.handle_config_changed(change).await?,
        }

        Ok(())
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

        self.on_state_enter(mode).await?;

        info!("Transitioned to {:?}", mode);
        Ok(())
    }

    async fn on_state_enter(&mut self, mode: SystemMode) -> SystemResult<()> {
        match mode {
            SystemMode::DeepSleep => {
                info!("Entering deep sleep mode");
                self.watchdog.disable().await?;
                self.stop().await?;
            }
            SystemMode::BleConnection => {
                info!("Entering BLE connection mode");
                self.watchdog.enable().await?;
                self.ble_service.start().await?;
            }
            SystemMode::NormalWork => {
                info!("Entering normal work mode");
                self.watchdog.enable().await?;
                self.execute_scheduled_tasks().await?;
            }
        }
        Ok(())
    }

    async fn on_state_exit(&mut self, mode: SystemMode) -> SystemResult<()> {
        match mode {
            SystemMode::DeepSleep => {
                info!("Exiting deep sleep mode");
                self.watchdog.enable().await?;
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

    pub async fn execute_scheduled_tasks(&mut self) -> SystemResult<()> {
        info!("Executing scheduled tasks");

        self.watchdog.start_task().await;

        let is_low_battery = self.power_manager.is_low_battery().await?;

        let config = self
            .config
            .ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;

        let current_time = self.time_service.get_solar_time().await?;
        let current_hour = current_time.get_hour() as u8;
        let current_minute = current_time.get_minute() as u8;

        if config.time_config.hour_chime_enabled {
            let last_chime_hour = self.last_chime_hour.unwrap_or(255);
            if last_chime_hour != current_hour && (current_minute == 0 || current_minute == 59) {
                info!("Playing hour chime for {}", current_hour);
                self.last_chime_hour = Some(current_hour);
                self.audio_service.play_hour_chime().await?;
            }
        }

        info!("Updating display data");
        self.watchdog.feed();

        let is_configured = self.ble_service.is_configured().await?;

        if !is_configured {
            let ssid = self.ble_service.get_device_name().await?;
            self.display_service.show_qrcode(ssid.as_str()).await?;
        } else {
            let is_need_sync =
                self.last_sync_time.is_none() || (current_hour == 0 || current_hour == 12);

            if is_need_sync {
                info!("Syncing network data (time, weather, quote)");
                match self.network_service.sync().await {
                    Ok(result) => {
                        info!(
                            "Sync completed: time={}, weather={}",
                            result.time_synced, result.weather_synced
                        );
                        self.last_sync_time =
                            Some(embassy_time::Instant::now().elapsed().as_secs());
                    }
                    Err(e) => {
                        error!("Sync failed: {:?}", e);
                    }
                }
            }

            let mut display_manager = DisplayManager::with_network_service(
                self.time_service,
                self.display_service,
                self.quote_service,
                self.network_service,
            );
            display_manager.update_display(is_low_battery).await?;
        }

        self.watchdog.feed();

        let next_wakeup = self.time_service.calculate_next_wakeup_time(config).await?;
        if let Some((timestamp, source)) = &next_wakeup {
            info!(
                "Next wakeup scheduled at: {:?}, source: {:?}",
                timestamp, source
            );
        }

        self.watchdog.end_task().await;

        if let Some((timestamp, _source)) = &next_wakeup {
            let current_ts = self.time_service.get_timestamp().await?;
            if *timestamp > current_ts {
                let duration = Duration::from_millis((*timestamp - current_ts) * 1000);
                info!("Entering light sleep for {:?}", duration);
                self.time_service.enter_light_sleep().await;
            }
        }

        info!("Scheduled tasks completed");

        Ok(())
    }

    pub async fn wait_for_event(&mut self) -> SystemResult<SystemEvent> {
        Ok(self.event_channel.receive().await)
    }

    pub async fn schedule_next_wakeup(&mut self) -> SystemResult<()> {
        if let Some(ref config) = self.config {
            if let Some((timestamp, _source)) =
                self.time_service.calculate_next_wakeup_time(config).await?
            {
                info!("Setting RTC alarm for timestamp: {:?}", timestamp);
                self.time_service.set_rtc_alarm(timestamp).await?;
            }
        }
        Ok(())
    }

    fn can_transition(&self, from: SystemMode, to: SystemMode) -> bool {
        if from == to {
            return true;
        }
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
                info!("BLE config received, syncing data");
            }
        }
        Ok(())
    }

    async fn handle_time_event(&mut self, event: TimeEvent) -> SystemResult<()> {
        match event {
            TimeEvent::MinuteTick => {
                debug!("Minute tick - not handled, rely on RTC wakeup");
            }
            TimeEvent::HourChimeTrigger => {
                debug!("HourChimeTrigger - handled in execute_scheduled_tasks");
            }
            TimeEvent::AlarmTrigger(_) => {
                debug!("AlarmTrigger - handled in execute_scheduled_tasks");
            }
        }
        Ok(())
    }

    fn get_last_chime_hour(&self) -> Option<u8> {
        self.last_chime_hour
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

    async fn handle_config_changed(&mut self, change: ConfigChange) -> SystemResult<()> {
        info!("Config changed: {:?}", change);
        match change {
            ConfigChange::TimeConfig => {
                if let Some(ref config) = self.config {
                    let current_time = self.time_service.get_solar_time().await?;
                    info!(
                        "Checking hour chime after config change, current time: {:02}:{:02}:{:02}",
                        current_time.get_hour(),
                        current_time.get_minute(),
                        current_time.get_second()
                    );
                }
            }
            ConfigChange::NetworkConfig => {
                info!("Network config changed");
            }
            ConfigChange::DisplayConfig => {
                info!("Display config changed");
            }
            ConfigChange::PowerConfig => {
                info!("Power config changed");
            }
            ConfigChange::LogConfig => {
                info!("Log config changed");
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
