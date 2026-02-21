use embassy_time::Duration;
use lxx_calendar_common::*;

use crate::managers::{DisplayManager, WatchdogManager};
use crate::services::{
    audio_service::AudioService, ble_service::BLEService, network_sync_service::NetworkSyncService,
    power_service::PowerManager, quote_service::QuoteService, time_service::TimeService,
};

pub struct StateManager<'a, P: PlatformTrait> {
    event_channel: LxxChannelReceiver<'a, SystemEvent>,
    current_state: SystemMode,
    time_service: TimeService<P::RtcDevice>,
    quote_service: QuoteService,
    ble_service: BLEService,
    power_manager: PowerManager<P::BatteryDevice>,
    audio_service: AudioService<P::AudioDevice>,
    network_sync_service: NetworkSyncService,
    watchdog: WatchdogManager<P::WatchdogDevice>,
    config: Option<&'a lxx_calendar_common::SystemConfig>,
    last_chime_hour: Option<u8>,
    last_sync_time: Option<u64>,
    is_charging: bool,
    low_battery_blocked: bool,
}

impl<'a, P: PlatformTrait> StateManager<'a, P> {
    pub fn new(
        event_receiver: LxxChannelReceiver<'a, SystemEvent>,
        time_service: TimeService<P::RtcDevice>,
        quote_service: QuoteService,
        ble_service: BLEService,
        power_manager: PowerManager<P::BatteryDevice>,
        audio_service: AudioService<P::AudioDevice>,
        network_sync_service: NetworkSyncService,
        watchdog_device: P::WatchdogDevice,
    ) -> Self {
        Self {
            current_state: SystemMode::LightSleep,
            event_channel: event_receiver,
            time_service,
            quote_service,
            ble_service,
            power_manager,
            audio_service,
            network_sync_service,
            watchdog: WatchdogManager::new(watchdog_device),
            config: None,
            last_chime_hour: None,
            last_sync_time: None,
            is_charging: false,
            low_battery_blocked: false,
        }
    }

    pub fn with_config(&mut self, config: &'a lxx_calendar_common::SystemConfig) {
        self.config = Some(config);
        self.last_chime_hour = None;
        self.last_sync_time = None;
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        self.time_service.initialize().await?;
        self.quote_service.initialize().await?;
        self.ble_service.initialize().await?;
        self.power_manager.initialize().await?;
        self.audio_service.initialize().await?;
        self.network_sync_service.initialize().await?;

        info!("All services initialized");

        info!("Initializing state manager");
        self.watchdog.initialize().await?;
        Ok(())
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
        info!("Transitioning from {:?} to {:?}", self.current_state, mode);

        self.on_state_exit(self.current_state).await?;

        self.current_state = mode;

        self.on_state_enter(mode).await?;

        info!("Transitioned to {:?}", mode);
        Ok(())
    }

    async fn on_state_enter(&mut self, mode: SystemMode) -> SystemResult<()> {
        match mode {
            SystemMode::LightSleep => {
                info!("Entering light sleep mode");
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
            SystemMode::LightSleep => {
                info!("Exiting light sleep mode");
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
        let charging = self.power_manager.is_charging().await?;
        let voltage = self.power_manager.get_voltage().await.ok();

        let config = self
            .config
            .ok_or_else(|| SystemError::HardwareError(HardwareError::NotInitialized))?;

        let current_time = self.time_service.get_solar_time().await?;
        let current_hour = current_time.get_hour() as u8;
        let current_minute = current_time.get_minute() as u8;

        if config.time_config.hour_chime_enabled && !self.low_battery_blocked {
            let last_chime_hour = self.last_chime_hour.unwrap_or(255);
            if last_chime_hour != current_hour && (current_minute == 0 || current_minute == 59) {
                info!("Playing hour chime for {}", current_hour);
                self.last_chime_hour = Some(current_hour);
                self.audio_service.play_hour_chime().await?;
            }
        } else if self.low_battery_blocked {
            debug!("Skipping hour chime due to low battery (not charging)");
        }

        info!("Updating display data");
        self.watchdog.feed();

        let is_configured = self.ble_service.is_configured().await?;

        if !is_configured {
            let ssid = self.ble_service.get_device_name().await?;
            let mut display_manager =
                DisplayManager::new(&mut self.time_service, &mut self.quote_service);
            display_manager.show_qrcode(ssid.as_str()).await?;
        } else {
            let is_need_sync =
                self.last_sync_time.is_none() || (current_hour == 0 || current_hour == 12);

            if is_need_sync && !self.low_battery_blocked {
                info!("Syncing network data (time, weather, quote)");
                match self.network_sync_service.sync(&mut self.time_service).await {
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
            } else if self.low_battery_blocked {
                debug!("Skipping network sync due to low battery (not charging)");
            }

            let mut display_manager = DisplayManager::with_network_sync_service(
                &mut self.time_service,
                &mut self.quote_service,
                &self.network_sync_service,
            );
            display_manager
                .update_display(is_low_battery, charging, voltage)
                .await?;
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
                self.time_service.set_rtc_alarm(*timestamp).await?;
                let duration = Duration::from_secs(*timestamp - current_ts);
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

    pub fn feed_watchdog(&mut self) {
        self.watchdog.feed();
    }

    async fn handle_wakeup_event(&mut self, event: WakeupEvent) -> SystemResult<()> {
        match event {
            WakeupEvent::WakeFromLightSleep => {
                info!("Waking from light sleep");
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
                if let Some(ref _config) = self.config {
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
            SystemStateEvent::EnterLightSleep => {
                info!("Entering light sleep");
                self.transition_to(SystemMode::LightSleep).await?;
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
            PowerEvent::ChargingStateChanged(charging) => {
                info!("Charging state changed: {}", charging);
                self.is_charging = charging;
                if !charging && self.low_battery_blocked {
                    info!("Not charging and low battery - operations remain blocked");
                } else if charging {
                    self.low_battery_blocked = false;
                    info!("Charging - low battery blocking cleared");
                }
            }
            PowerEvent::LowPowerModeChanged(enabled) => {
                info!("Low power mode changed: {}", enabled);
                if enabled && !self.is_charging {
                    self.low_battery_blocked = true;
                    warn!("Low battery detected and not charging - time sync and alarm blocked");
                } else {
                    self.low_battery_blocked = false;
                    info!("Low battery condition cleared");
                }
            }
        }
        Ok(())
    }
}
