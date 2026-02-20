use heapless::String;
use lxx_calendar_common::*;

use crate::services::{display_service::DisplayService, quote_service::QuoteService, time_service::TimeService};

pub struct DisplayManager<'a, R: Rtc> {
    time_service: &'a mut TimeService<R>,
    display_service: &'a mut DisplayService,
    quote_service: &'a mut QuoteService,
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
        }
    }

    pub async fn update_display(&mut self) -> SystemResult<()> {
        let current_time = self.time_service.get_current_time().await?;
        
        let lunar_date = match self.time_service.get_lunar_date().await {
            Ok(date) => date,
            Err(e) => {
                warn!("Failed to get lunar date: {:?}", e);
                LunarDate {
                    year: 0,
                    month: 0,
                    day: 0,
                    is_leap: false,
                    zodiac: "",
                    ganzhi_year: "",
                    ganzhi_month: "",
                    ganzhi_day: "",
                }
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

        let display_data = DisplayData {
            time: current_time,
            lunar_date,
            weather: None,
            quote,
            layout: DisplayLayout::Default,
        };

        self.display_service.update_display(display_data).await?;

        Ok(())
    }
}
