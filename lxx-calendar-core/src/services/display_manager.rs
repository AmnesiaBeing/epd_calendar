use heapless::String;
use lxx_calendar_common::*;

use crate::services::{
    display_service::DisplayService, network_sync_service::NetworkSyncService,
    quote_service::QuoteService, time_service::TimeService,
};

pub struct DisplayManager<'a, R: Rtc> {
    time_service: &'a mut TimeService<R>,
    display_service: &'a mut DisplayService,
    quote_service: &'a mut QuoteService,
    network_service: Option<&'a mut NetworkSyncService<R>>,
}

impl<'a, R: Rtc> DisplayManager<'a, R> {
    pub fn new(
        time_service: &'a mut TimeService<R>,
        display_service: &'a mut DisplayService,
        quote_service: &'a mut QuoteService,
    ) -> Self {
        Self {
            time_service,
            display_service,
            quote_service,
            network_service: None,
        }
    }

    pub fn with_network_service(
        time_service: &'a mut TimeService<R>,
        display_service: &'a mut DisplayService,
        quote_service: &'a mut QuoteService,
        network_service: &'a mut NetworkSyncService<R>,
    ) -> Self {
        Self {
            time_service,
            display_service,
            quote_service,
            network_service: Some(network_service),
        }
    }

    pub async fn update_display(&mut self, low_battery: bool) -> SystemResult<()> {
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
        };

        self.display_service.update_display(display_data).await?;

        Ok(())
    }
}
