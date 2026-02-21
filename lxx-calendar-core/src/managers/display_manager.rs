use embassy_time::Duration;
use heapless::String;
use lxx_calendar_common::*;

use crate::services::{
    network_sync_service::NetworkSyncService, quote_service::QuoteService,
    time_service::TimeService,
};

pub struct DisplayManager<'a, R: Rtc> {
    time_service: &'a mut TimeService<R>,
    quote_service: &'a mut QuoteService,
    network_service: Option<&'a NetworkSyncService>,
    state: RefreshState,
    current_layout: DisplayLayout,
    last_refresh_time: Option<u64>,
    current_display_data: Option<DisplayData>,
    refresh_interval_seconds: u16,
    low_power_mode: bool,
}

impl<'a, R: Rtc> DisplayManager<'a, R> {
    pub fn new(time_service: &'a mut TimeService<R>, quote_service: &'a mut QuoteService) -> Self {
        Self {
            time_service,
            quote_service,
            network_service: None,
            state: RefreshState::Idle,
            current_layout: DisplayLayout::Default,
            last_refresh_time: None,
            current_display_data: None,
            refresh_interval_seconds: 60,
            low_power_mode: false,
        }
    }

    pub fn with_network_sync_service(
        time_service: &'a mut TimeService<R>,
        quote_service: &'a mut QuoteService,
        network_sync_service: &'a NetworkSyncService,
    ) -> Self {
        Self {
            time_service,
            quote_service,
            network_service: Some(network_sync_service),
            state: RefreshState::Idle,
            current_layout: DisplayLayout::Default,
            last_refresh_time: None,
            current_display_data: None,
            refresh_interval_seconds: 60,
            low_power_mode: false,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing display manager");
        self.state = RefreshState::Idle;
        Ok(())
    }

    pub async fn update_display(
        &mut self,
        low_battery: bool,
        charging: bool,
        voltage: Option<u16>,
    ) -> SystemResult<()> {
        let solar_time = self.time_service.get_solar_time().await?;
        let weekday = self.time_service.get_weekday().await?;

        let lunar_date = match self.time_service.get_lunar_date().await {
            Ok(day) => day,
            Err(e) => {
                warn!("Failed to get lunar date: {:?}", e);
                use sxtwl_rs::lunar::LunarDay;
                LunarDay::from_ymd(2024, 1, 1)
            }
        };

        let quote = match self.quote_service.get_quote().await {
            Ok(q) => {
                let mut s = String::new();
                s.push_str(q.text).ok();
                Some(s)
            }
            Err(e) => {
                debug!("No quote available: {:?}", e);
                None
            }
        };

        let solar_term = match self.time_service.get_solar_term().await {
            Ok(term) => term,
            Err(e) => {
                warn!("Failed to get solar term: {:?}", e);
                None
            }
        };

        let solar_festival = match self.time_service.get_solar_festival().await {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to get solar festival: {:?}", e);
                None
            }
        };

        let lunar_festival = match self.time_service.get_lunar_festival().await {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to get lunar festival: {:?}", e);
                None
            }
        };

        let weather = match &mut self.network_service {
            Some(service) => match service.get_weather().await {
                Ok(w) => Some(w),
                Err(e) => {
                    warn!("Failed to get weather: {:?}", e);
                    None
                }
            },
            None => None,
        };

        let display_data = DisplayData {
            solar_time,
            weekday,
            lunar_date,
            weather,
            quote,
            layout: DisplayLayout::Default,
            solar_term,
            lunar_festival,
            solar_festival,
            low_battery,
            charging,
            voltage,
        };

        info!("Updating display data");
        self.current_display_data = Some(display_data);

        if self.state == RefreshState::Idle {
            self.refresh().await?;
        }

        Ok(())
    }

    pub async fn refresh(&mut self) -> SystemResult<()> {
        if self.state != RefreshState::Idle {
            info!("Display busy, skipping refresh");
            return Ok(());
        }

        info!("Refreshing display");

        let data = match self.current_display_data.clone() {
            Some(d) => d,
            None => {
                info!("No display data to refresh");
                return Ok(());
            }
        };

        self.state = RefreshState::SendingData;

        match self.render_to_framebuffer(&data).await {
            Ok(_) => {
                self.state = RefreshState::Refreshing;

                embassy_time::Timer::after(Duration::from_secs(10)).await;

                self.state = RefreshState::Idle;
                self.last_refresh_time = Some(embassy_time::Instant::now().elapsed().as_secs());
                info!("Display refreshed successfully");
            }
            Err(e) => {
                self.state = RefreshState::Error(RefreshError::CommunicationError);
                error!("Display refresh failed: {:?}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    async fn render_to_framebuffer(&mut self, data: &DisplayData) -> SystemResult<()> {
        info!(
            "Rendering: time={}-{:02}-{:02} {:02}:{:02}, low_battery={}",
            data.solar_time.get_year(),
            data.solar_time.get_month(),
            data.solar_time.get_day(),
            data.solar_time.get_hour(),
            data.solar_time.get_minute(),
            data.low_battery
        );
        Ok(())
    }

    pub async fn set_refresh_interval(&mut self, seconds: u16) -> SystemResult<()> {
        self.refresh_interval_seconds = seconds;
        Ok(())
    }

    pub async fn enter_low_power_mode(&mut self) -> SystemResult<()> {
        self.low_power_mode = true;
        info!("Display entered low power mode");
        Ok(())
    }

    pub async fn exit_low_power_mode(&mut self) -> SystemResult<()> {
        self.low_power_mode = false;
        info!("Display exited low power mode");
        Ok(())
    }

    pub async fn show_qrcode(&mut self, ssid: &str) -> SystemResult<()> {
        info!("Showing QR code for SSID: {}", ssid);
        self.current_layout = DisplayLayout::LargeTime;
        Ok(())
    }
}
