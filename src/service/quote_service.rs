// src/service/quote_service.rs
use crate::assets::generated_hitokoto_data::{HITOKOTOS, Hitokoto};
use crate::common::error::{AppError, Result};
use crate::driver::lcg::Lcg;

pub struct QuoteService {
    last_index: Option<usize>,
}

impl QuoteService {
    pub fn new() -> Self {
        Self { last_index: None }
    }

    pub async fn get_random_quote(&self) -> Result<&Hitokoto> {
        if HITOKOTOS.is_empty() {
            return Err(AppError::QuoteError);
        }

        let mut lcg = Lcg::new();
        let index = lcg.next_index(HITOKOTOS.len());
        let hitokoto = &HITOKOTOS[index];

        log::debug!(
            "Selected hitokoto at index {}: {}",
            index,
            &hitokoto.hitokoto[..hitokoto.hitokoto.len().min(30)]
        );

        Ok(hitokoto)
    }
}

impl Default for QuoteService {
    fn default() -> Self {
        Self::new()
    }
}
