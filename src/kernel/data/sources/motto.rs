//! 格言数据源模块
//! 提供格言相关数据的数据源实现

use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::Duration;
use heapless::format;

use crate::assets::generated_hitokoto_data::{FROM_STRINGS, FROM_WHO_STRINGS, HITOKOTOS};
use crate::common::error::{AppError, Result};
use crate::common::{GlobalMutex, GlobalRwLockWriteGuard};
use crate::kernel::data::DataSource;
use crate::kernel::data::types::{
    CacheKey, CacheKeyValueMap, DynamicValue, alloc_string_to_heapless,
};
use crate::kernel::system::api::DefaultSystemApi;

// --------------- 常量定义 ---------------
const KEY_LENGTH: usize = 32;
const CACHE_KEY_PREFIX: &str = "hitokoto";
const CACHE_KEY_CONTENT: &str = "content";
const CACHE_KEY_AUTHOR: &str = "author";
const CACHE_KEY_FROM: &str = "from";

/// 格言数据源结构体
pub struct MottoDataSource {}

impl MottoDataSource {
    /// 创建新的格言数据源实例
    pub fn new() -> Result<Self> {
        log::info!("正在创建格言数据源实例");
        Ok(Self {})
    }

    /// 构建全局缓存key（拼接前缀：hitokoto.xxx）
    fn build_cache_key(field: &str) -> Result<CacheKey> {
        log::debug!("正在为字段 {} 构建缓存key", field);
        let full_key = format!(KEY_LENGTH; "{}.{}", CACHE_KEY_PREFIX, field).map_err(|_| {
            log::error!("格言缓存key太长: {}", field);
            AppError::InvalidFieldName
        })?;
        log::debug!("成功构建缓存key: {}", full_key);
        Ok(full_key)
    }

    /// 安全写入全局缓存字段
    fn write_cache_field(
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
        key: &str,
        value: DynamicValue,
    ) -> Result<()> {
        let cache_key = Self::build_cache_key(key)?;
        log::debug!("正在将字段 {} 写入缓存，值为: {:?}", key, value);
        cache_guard.insert(cache_key, value);
        log::info!("成功将字段 {} 写入缓存", key);
        Ok(())
    }

    /// 获取随机格言
    fn get_random_motto(&mut self) -> Result<(&'static str, &'static str, &'static str)> {
        log::info!("正在获取随机格言");
        // 简单的伪随机数算法
        let random_u64 = getrandom::u64().map_err(|e| {
            log::error!("获取随机数失败: {:?}", e);
            AppError::RandomGenerationFailed
        })?;
        log::debug!("生成的随机数: {}", random_u64);

        let index = random_u64 as usize % HITOKOTOS.len();
        log::debug!(
            "选择的格言索引: {} (总共有 {} 条格言)",
            index,
            HITOKOTOS.len()
        );

        // 获取格言数据
        let hitokoto = &HITOKOTOS[index];

        // 获取来源和作者
        let from = FROM_STRINGS.get(hitokoto.from as usize).unwrap_or(&"");
        let from_who = FROM_WHO_STRINGS
            .get(hitokoto.from_who as usize)
            .unwrap_or(&"");

        log::info!("成功获取随机格言:  作者='{}', 来源='{}'", from_who, from);
        Ok((hitokoto.hitokoto, from_who, from))
    }
}

#[async_trait(?Send)]
impl DataSource for MottoDataSource {
    /// 获取数据源名称
    fn name(&self) -> &'static str {
        CACHE_KEY_PREFIX
    }

    /// 刷新数据并直接写入缓存
    async fn refresh_with_cache(
        &mut self,
        _system_api: &'static GlobalMutex<DefaultSystemApi>,
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
    ) -> Result<()> {
        log::info!("开始刷新格言数据源");

        // 获取随机格言
        let (content, author, from) = self.get_random_motto()?;

        // 将数据写入缓存
        log::info!("开始将格言数据写入缓存");

        Self::write_cache_field(
            cache_guard,
            CACHE_KEY_CONTENT,
            DynamicValue::String(alloc_string_to_heapless(content)?),
        )?;

        Self::write_cache_field(
            cache_guard,
            CACHE_KEY_AUTHOR,
            DynamicValue::String(alloc_string_to_heapless(author)?),
        )?;

        Self::write_cache_field(
            cache_guard,
            CACHE_KEY_FROM,
            DynamicValue::String(alloc_string_to_heapless(from)?),
        )?;

        log::info!("格言数据源刷新完成");
        Ok(())
    }

    /// 获取刷新间隔（每12小时刷新一次）
    fn refresh_interval(&self) -> Duration {
        Duration::from_secs(12 * 60 * 60)
    }
}
