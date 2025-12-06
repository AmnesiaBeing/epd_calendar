// src/service/quote_service.rs

//! 名言服务模块 - 提供随机名言获取功能
//!
//! 该模块从预生成的名言数据中随机选择一条名言，确保每次选择不同的名言。

use crate::assets::generated_hitokoto_data::HITOKOTOS;
use crate::common::Hitokoto;
use crate::common::error::{AppError, Result};

/// 名言服务，提供随机名言获取功能
pub struct QuoteService {
    /// 上次选择的名言索引，用于避免连续选择相同的名言
    last_index: Option<usize>,
}

impl QuoteService {
    /// 创建新的名言服务实例
    ///
    /// # 返回值
    /// 返回新的QuoteService实例
    pub fn new() -> Self {
        Self { last_index: None }
    }

    /// 获取随机名言
    ///
    /// # 返回值
    /// - `Result<&'static Hitokoto>`: 成功返回名言引用，失败返回错误
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
    /// 默认实现，创建新的名言服务实例
    ///
    /// # 返回值
    /// 返回新的QuoteService实例
    fn default() -> Self {
        Self::new()
    }
}
