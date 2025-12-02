// src/service/quote_service.rs
use crate::assets::generated_hitokoto_data::{HITOKOTOS, Hitokoto};
use crate::common::error::{AppError, Result};

pub struct QuoteService {
    last_index: Option<usize>,
}

impl QuoteService {
    pub fn new() -> Self {
        Self { last_index: None }
    }

    pub async fn get_random_quote(&self) -> Result<&'static Hitokoto> {
        if HITOKOTOS.is_empty() {
            return Err(AppError::QuoteError);
        }

        // 生成的索引和之前生成的索引不同
        let index = loop {
            let seed = getrandom::u64().map_err(|_| AppError::NetworkStackInitFailed)?;
            let index = seed as usize % HITOKOTOS.len();
            if let Some(last_index) = self.last_index {
                if index != last_index {
                    break index;
                }
            } else {
                break index;
            }
        };

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
