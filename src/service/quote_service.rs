// src/service/quote_service.rs
use crate::assets::quotes::QUOTES;
use crate::common::error::{AppError, Result};
use log::debug;

pub struct QuoteService {
    // 可以添加缓存、上次选择的索引等状态
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

        // 简单的随机选择（在实际嵌入式系统中可能需要更简单的随机算法）
        let index = self.get_random_index(QUOTES.len());
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

    fn get_random_index(&self, max: usize) -> usize {
        // 简化的"随机"算法 - 使用系统时间的微秒部分
        // 在实际嵌入式系统中，可以使用硬件RNG或更简单的算法
        #[cfg(feature = "std")]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            (duration.as_micros() as usize) % max
        }

        #[cfg(not(feature = "std"))]
        {
            // 对于no_std环境，使用简单的线性同余生成器或固定序列
            // 这里使用一个简单的基于时间的伪随机
            let timestamp = embassy_time::Instant::now().as_micros();
            (timestamp as usize) % max
        }
    }
}

impl Default for QuoteService {
    fn default() -> Self {
        Self::new()
    }
}
