use embassy_time::Duration;
use lxx_calendar_common::*;

use crate::managers::{ConfigManager, DisplayManager, WatchdogManager};
use crate::services::{
    audio_service::AudioService, ble_service::BLEService, button_service::ButtonService,
    network_sync_service::NetworkSyncService, power_service::PowerManager,
    quote_service::QuoteService, time_service::TimeService,
};

pub struct StateManager<'a, P: PlatformTrait, F: lxx_calendar_common::storage::FlashDevice> {
    event_channel: LxxChannelReceiver<'a, SystemEvent>,
    event_sender: LxxChannelSender<'static, SystemEvent>,
    current_state: SystemMode,
    time_service: TimeService<P::RtcDevice>,
    quote_service: QuoteService,
    ble_service: BLEService<P::BLEDevice>,
    power_manager: PowerManager<P::BatteryDevice>,
    audio_service: AudioService<P::AudioDevice>,
    network_sync_service: NetworkSyncService,
    wifi_device: P::WifiDevice,
    button_service: ButtonService<P::ButtonDevice>,
    watchdog: WatchdogManager<P::WatchdogDevice>,
    config_manager: ConfigManager<F>,
    last_chime_hour: Option<u8>,
    last_sync_time: Option<u64>,
    is_charging: bool,
    low_battery_blocked: bool,
    alarm_active: bool,
    last_alarm_check: Option<(u8, u8)>,
}

impl<'a, P: PlatformTrait, F: lxx_calendar_common::storage::FlashDevice> StateManager<'a, P, F> {
    pub fn new(
        event_receiver: LxxChannelReceiver<'a, SystemEvent>,
        event_sender: LxxChannelSender<'static, SystemEvent>,
        button_service: ButtonService<P::ButtonDevice>,
        time_service: TimeService<P::RtcDevice>,
        quote_service: QuoteService,
        ble_service: BLEService<P::BLEDevice>,
        power_manager: PowerManager<P::BatteryDevice>,
        audio_service: AudioService<P::AudioDevice>,
        network_sync_service: NetworkSyncService,
        wifi_device: P::WifiDevice,
        watchdog_device: P::WatchdogDevice,
        config_manager: ConfigManager<F>,
    ) -> Self {
        Self {
            current_state: SystemMode::LightSleep,
            event_channel: event_receiver,
            event_sender,
            button_service,
            alarm_active: false,
            last_alarm_check: None,
            time_service,
            quote_service,
            ble_service,
            power_manager,
            audio_service,
            network_sync_service,
            wifi_device,
            watchdog: WatchdogManager::new(watchdog_device),
            config_manager,
            last_chime_hour: None,
            last_sync_time: None,
            is_charging: false,
            low_battery_blocked: false,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        self.config_manager.initialize().await?;
        
        let config = self.config_manager.load_config().await?;
        info!(
            "Configuration loaded, hour_chime_enabled: {}",
            config.time_config.hour_chime_enabled
        );
        
        self.time_service.initialize().await?;
        self.quote_service.initialize().await?;
        self.ble_service
            .initialize(self.event_sender.clone())
            .await?;
        self.power_manager.initialize().await?;
        self.audio_service.initialize().await?;
        self.network_sync_service.initialize().await?;
        self.button_service.initialize().await?;

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
            SystemEvent::BLEEvent(evt) => self.handle_ble_event(evt).await?,
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

    fn matches_repeat_day(repeat_days: u8, weekday: u8) -> bool {
        if repeat_days == 0 {
            return true; // 未设置重复，默认触发
        }
        // repeat_days 是位掩码，bit 0 = 周日, bit 1 = 周一, ..., bit 6 = 周六
        (repeat_days & (1 << weekday)) != 0
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

        let config = self.config_manager.get_config()
            .map_err(|_| SystemError::HardwareError(HardwareError::NotInitialized))?;

        let current_time = self.time_service.get_solar_time().await?;
        let current_hour = current_time.get_hour() as u8;
        let current_minute = current_time.get_minute() as u8;

        // 检查闹钟
        if !self.low_battery_blocked {
            // 获取星期几 (0=周日, 1=周一, ..., 6=周六)
            let solar_day = sxtwl_rs::solar::SolarDay::from_ymd(
                current_time.get_year() as isize,
                current_time.get_month() as usize,
                current_time.get_day() as usize,
            );
            let week = solar_day.get_week();
            let current_weekday = week.get_index() as u8;
            
            for alarm in &config.time_config.alarms {
                if alarm.enabled 
                    && alarm.hour == current_hour 
                    && alarm.minute == current_minute
                    && Self::matches_repeat_day(alarm.repeat_days, current_weekday)
                {
                    if self.last_alarm_check != Some((current_hour, current_minute)) {
                        info!("Alarm triggered at {:02}:{:02}", alarm.hour, alarm.minute);
                        self.last_alarm_check = Some((current_hour, current_minute));
                        
                        // 发送闹钟事件
                        let _ = self.event_sender.try_send(SystemEvent::TimeEvent(
                            TimeEvent::AlarmTrigger(*alarm)
                        ));
                    }
                }
            }
        }

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

        let next_wakeup = self.time_service.calculate_next_wakeup_time(&config).await?;
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
        let config = self.config_manager.get_config()
            .map_err(|_| SystemError::HardwareError(HardwareError::NotInitialized))?;
        
        if let Some((timestamp, _source)) =
            self.time_service.calculate_next_wakeup_time(&config).await?
        {
            info!("Setting RTC alarm for timestamp: {:?}", timestamp);
            self.time_service.set_rtc_alarm(timestamp).await?;
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
        // 如果闹钟正在响，任何按键都停止闹钟
        if self.alarm_active {
            info!("Stopping alarm due to user interaction");
            self.alarm_active = false;
            return Ok(());
        }
        
        match event {
            UserEvent::ButtonDoubleClick => {
                info!("Button double click - No function yet");
            }
            UserEvent::ButtonTripleClick => {
                info!("Button triple click detected - Entering pairing mode");
                self.transition_to(SystemMode::BleConnection).await?;

                let is_configured = self.ble_service.is_configured().await?;
                if !is_configured {
                    let ssid = self.ble_service.get_device_name().await?;
                    info!("Showing QR code for pairing: {}", ssid);

                    let mut display_manager = crate::managers::DisplayManager::new(
                        &mut self.time_service,
                        &mut self.quote_service,
                    );
                    display_manager.show_qrcode(ssid.as_str()).await?;
                }
            }
            UserEvent::ButtonShortPress => {
                info!("Button short press");
                if self.current_state == SystemMode::NormalWork {
                    self.transition_to(SystemMode::BleConnection).await?;
                }
            }
            UserEvent::ButtonLongPress => {
                info!("Button long press detected (>15s) - Restoring factory defaults");
                self.transition_to(SystemMode::BleConnection).await?;

                info!("Factory reset triggered");
                info!("This feature is not yet implemented");
                info!("TODO: Clear configuration and reboot");
            }
        }
        Ok(())
    }

    async fn handle_ble_event(&mut self, event: BLEEvent) -> SystemResult<()> {
        match event {
            BLEEvent::WifiConfigReceived { ssid, password } => {
                info!("WiFi config received: ssid={}", ssid);
                
                // 保存配置到 Flash
                let ssid_clone = ssid.clone();
                let password_clone = password.clone();
                self.config_manager.update_config(|config| {
                    config.network_config.wifi_ssid = ssid_clone.clone();
                    let mut pwd_bytes = heapless::Vec::new();
                    let _ = pwd_bytes.extend_from_slice(password_clone.as_bytes());
                    config.network_config.wifi_password.data = pwd_bytes;
                }).await?;
                info!("WiFi config saved to flash");
                
                // 连接 WiFi
                self.network_sync_service.save_wifi_config(ssid, password);
                if let Err(e) = self
                    .network_sync_service
                    .connect_wifi(&mut self.wifi_device)
                    .await
                {
                    error!("WiFi connection failed: {:?}", e);
                } else {
                    info!("WiFi connected, starting network sync");
                    let result = self.network_sync_service.sync(&mut self.time_service).await;
                    match result {
                        Ok(_) => info!("Network sync completed successfully"),
                        Err(e) => error!("Network sync failed: {:?}", e),
                    }
                }
            }
            BLEEvent::NetworkConfigReceived {
                location_id,
                latitude: _,
                longitude: _,
                location_name: _,
                sync_interval_minutes,
                auto_sync: _,
            } => {
                info!(
                    "Network config received: location_id={:?}, sync_interval={}",
                    location_id, sync_interval_minutes
                );
                
                self.config_manager.update_config(|config| {
                    config.network_config.location_id = location_id.clone();
                    config.network_config.sync_interval_minutes = sync_interval_minutes;
                }).await?;
                
                info!("Network config saved to flash");
            }
            BLEEvent::DisplayConfigReceived {
                refresh_interval_seconds,
                low_power_refresh_enabled,
            } => {
                info!(
                    "Display config received: refresh={}, low_power={}",
                    refresh_interval_seconds, low_power_refresh_enabled
                );
                
                self.config_manager.update_config(|config| {
                    config.display_config.refresh_interval_seconds = refresh_interval_seconds;
                    config.display_config.low_power_refresh_enabled = low_power_refresh_enabled;
                }).await?;
                
                info!("Display config saved to flash");
            }
            BLEEvent::TimeConfigReceived {
                timezone_offset,
                hour_chime_enabled,
            } => {
                info!(
                    "Time config received: timezone_offset={}, hour_chime_enabled={}",
                    timezone_offset, hour_chime_enabled
                );
                
                self.config_manager.update_config(|config| {
                    config.time_config.timezone_offset = timezone_offset;
                    config.time_config.hour_chime_enabled = hour_chime_enabled;
                }).await?;
                
                info!("Time config saved to flash");
            }
            BLEEvent::PowerConfigReceived {
                low_power_mode_enabled,
            } => {
                info!(
                    "Power config received: low_power_mode_enabled={}",
                    low_power_mode_enabled
                );
                
                self.config_manager.update_config(|config| {
                    config.power_config.low_power_mode_enabled = low_power_mode_enabled;
                }).await?;
                
                info!("Power config saved to flash");
            }
            BLEEvent::LogConfigReceived {
                log_level,
                log_to_flash,
            } => {
                info!(
                    "Log config received: level={:?}, to_flash={}",
                    log_level, log_to_flash
                );
                
                self.config_manager.update_config(|config| {
                    config.log_config.log_level = log_level;
                    config.log_config.log_to_flash = log_to_flash;
                }).await?;
                
                info!("Log config saved to flash");
            }
            BLEEvent::CommandNetworkSync => {
                info!("Command: network sync");
                let result = self.network_sync_service.sync(&mut self.time_service).await;
                match result {
                    Ok(_) => info!("Network sync completed successfully"),
                    Err(e) => error!("Network sync failed: {:?}", e),
                }
            }
            BLEEvent::CommandReboot => {
                info!("Command: reboot");
            }
            BLEEvent::CommandFactoryReset => {
                info!("Command: factory reset");
                
                match self.config_manager.factory_reset().await {
                    Ok(_) => {
                        info!("Factory reset completed successfully");
                    }
                    Err(e) => {
                        error!("Factory reset failed: {:?}", e);
                    }
                }
            }
            BLEEvent::OTAStart => {
                info!("OTA start");
            }
            BLEEvent::OTAData(data) => {
                info!("OTA data: {} bytes", data.len());
            }
            BLEEvent::OTAComplete => {
                info!("OTA complete");
            }
            BLEEvent::OTACancel => {
                info!("OTA cancel");
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
            TimeEvent::AlarmTrigger(alarm_info) => {
                info!("Alarm triggered: {:02}:{:02}", alarm_info.hour, alarm_info.minute);
                
                self.alarm_active = true;
                
                // 播放闹钟声音 (5秒)
                for i in 0..10 {
                    if !self.alarm_active {
                        info!("Alarm stopped by user");
                        break;
                    }
                    
                    info!("Alarm beep {} / 10", i + 1);
                    self.audio_service.play_tone(800, 400).await?;
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
                }
                
                self.alarm_active = false;
                info!("Alarm finished");
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
        
        let config = self.config_manager.get_config()
            .map_err(|_| SystemError::HardwareError(HardwareError::NotInitialized))?;
        
        match change {
            ConfigChange::TimeConfig => {
                let current_time = self.time_service.get_solar_time().await?;
                info!(
                    "Time config changed, current time: {:02}:{:02}:{:02}",
                    current_time.get_hour(),
                    current_time.get_minute(),
                    current_time.get_second()
                );
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
        
        info!("Saving config to flash");
        self.config_manager.save_config(config).await?;
        info!("Config saved successfully");
        
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
