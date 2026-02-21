use lxx_calendar_common::*;
use lxx_calendar_quotes::Quote;

pub struct QuoteService {
    initialized: bool,
    today_quote: Option<Quote<'static>>,
}

impl QuoteService {
    pub fn new() -> Self {
        Self {
            initialized: false,
            today_quote: None,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing quote service");

        self.initialized = true;

        info!("Quote service initialized");
        Ok(())
    }

    pub async fn get_quote(&mut self) -> SystemResult<Quote<'static>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if self.today_quote.is_none() {
            info!("No quote available, refreshing");
            self.refresh().await?;
        }

        self.today_quote
            .ok_or_else(|| SystemError::DataError(DataError::NotFound))
    }

    pub async fn refresh(&mut self) -> SystemResult<Quote<'static>> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        let quote = self.get_random_quote()?;

        self.today_quote = Some(quote);

        info!(
            "Quote refreshed: {}",
            self.today_quote.as_ref().unwrap().text
        );

        Ok(quote)
    }

    fn get_random_quote(&self) -> SystemResult<Quote<'static>> {
        let count = lxx_calendar_quotes::get_quote_count();

        if count == 0 {
            return Err(SystemError::DataError(DataError::NotFound));
        }

        let index = Self::random_index(count);

        lxx_calendar_quotes::get_daily_quote(index as u16)
            .ok_or_else(|| SystemError::DataError(DataError::NotFound))
    }

    fn random_index(max: usize) -> usize {
        let mut buf = [0u8; 4];
        let _ = getrandom::fill(&mut buf);
        let random_u32 = u32::from_le_bytes(buf);
        (random_u32 as usize) % max
    }
}
