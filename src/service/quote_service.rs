// src/service/quote_service.rs
use crate::assets::quotes::QUOTES;
use crate::common::error::{AppError, Result};
use crate::driver::lcg::Lcg;

pub struct QuoteService {
    last_index: Option<usize>,
}

impl QuoteService {
    pub fn new() -> Self {
        Self { last_index: None }
    }

    pub async fn get_random_quote(&self) -> Result<String> {
        if QUOTES.is_empty() {
            return Err(AppError::QuoteError);
        }

        let lcg = Lcg::new();
        let index = lcg.next_index(QUOTES.len());
        let quote = QUOTES[index].to_string();

        debug!(
            "Selected quote at index {}: {}",
            index,
            &quote[..quote.len().min(30)]
        );

        Ok(quote)
    }

    pub async fn get_quote_by_index(&self, index: usize) -> Result<String> {
        QUOTES
            .get(index)
            .map(|s| s.to_string())
            .ok_or(AppError::QuoteError)
    }

    pub fn get_quotes_count(&self) -> usize {
        QUOTES.len()
    }
}

impl Default for QuoteService {
    fn default() -> Self {
        Self::new()
    }
}
