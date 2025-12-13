//! 配置数据源模块
//! 实现配置数据的加载、缓存和访问接口

use alloc::boxed::Box;
use async_trait::async_trait;
use core::str::FromStr;
use embassy_time::Duration;
use heapless::format;

use crate::common::error::{AppError, Result};
use crate::common::{GlobalMutex, GlobalRwLockReadGuard, GlobalRwLockWriteGuard};
use crate::kernel::data::DataSource;
use crate::kernel::data::DynamicValue;
use crate::kernel::data::types::{CacheKeyValueMap, HeaplessString, HeaplessVec, KEY_LENGTH};
use crate::kernel::system::api::{ConfigApi, DefaultSystemApi, SystemApi};

// ======================== 常量定义 ========================
/// 配置缓存key前缀
const CONFIG_CACHE_PREFIX: &str = "config";
/// 最大配置项数量
const MAX_CONFIG_ITEMS: usize = 10;

// ======================== 类型别名（增强可读性） ========================
/// 配置项类型（字段名+值）
type ConfigItem = (HeaplessString<KEY_LENGTH>, DynamicValue);
/// 配置项列表（固定长度，嵌入式友好）
type ConfigItemList = HeaplessVec<ConfigItem, MAX_CONFIG_ITEMS>;

// ======================== 配置数据源结构体 ========================
pub struct ConfigDataSource {
    /// 系统API实例（全局互斥锁保护）
    system_api: &'static GlobalMutex<DefaultSystemApi>,
    /// 配置是否被修改（脏标记）
    dirty: bool,
}

impl ConfigDataSource {
    /// 创建新的配置数据源实例
    pub async fn new(system_api: &'static GlobalMutex<DefaultSystemApi>) -> Result<Self> {
        Ok(Self {
            system_api,
            dirty: false,
        })
    }

    // ======================== 核心辅助方法 ========================
    /// 构建配置缓存key（config.字段名）
    fn build_cache_key(&self, field: &str) -> Result<HeaplessString<KEY_LENGTH>> {
        let cache_key = format!(KEY_LENGTH; "{}.{}", CONFIG_CACHE_PREFIX, field).map_err(|_| {
            log::error!("Config cache key too long: {}", field);
            AppError::InvalidFieldName
        })?;
        Ok(cache_key)
    }

    /// 字符串转HeaplessString（统一错误处理）
    fn str_to_heapless(&self, s: &str) -> Result<HeaplessString<KEY_LENGTH>> {
        HeaplessString::from_str(s).map_err(|_| {
            log::error!("Failed to convert string to heapless: {}", s);
            AppError::InvalidFieldName
        })
    }

    // ======================== 存储交互逻辑 ========================
    /// 从存储加载配置
    async fn load_config_from_storage(&mut self) -> Result<Option<ConfigItemList>> {
        // 读取存储数据
        let config_data = {
            let system_api_guard = self.system_api.lock().await;
            let config_api = system_api_guard.get_config_api();
            config_api.read_config().await?
        };

        // 解析配置
        match config_data {
            Some(data) => {
                let config = postcard::from_bytes::<ConfigItemList>(&data).map_err(|_| {
                    log::error!("Failed to parse config from storage");
                    AppError::InvalidConfigData
                })?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// 保存配置到存储（从全局缓存读取所有配置字段）
    async fn save_config_to_storage(
        &mut self,
        cache_guard: &GlobalRwLockReadGuard<'_, CacheKeyValueMap>,
    ) -> Result<()> {
        // 从缓存构建配置列表
        let mut config = ConfigItemList::new();

        // 遍历所有缓存项，找出配置相关的字段
        for (cache_key, value) in cache_guard.iter() {
            if let Some(field) = cache_key.as_str().strip_prefix("config.") {
                config
                    .push((self.str_to_heapless(field)?, value.clone()))
                    .map_err(|_| {
                        log::error!("Failed to push config item for storage: {}", field);
                        AppError::InvalidConfigData
                    })?;
            }
        }

        // 序列化并写入存储
        let data = postcard::to_allocvec(&config).map_err(|_| {
            log::error!("Failed to serialize config for storage");
            AppError::InvalidConfigData
        })?;

        let system_api_guard = self.system_api.lock().await;
        let config_api = system_api_guard.get_config_api();
        config_api.write_config(&data).await?;
        drop(system_api_guard);

        // 清除脏标记
        self.dirty = false;
        log::debug!("Config saved to storage ({} fields)", config.len());
        Ok(())
    }

    // ======================== 缓存操作接口 ========================
    /// 设置配置字段（直接写入全局缓存）
    pub async fn set_config_field(
        &mut self,
        cache_guard: &mut GlobalRwLockWriteGuard<'_, CacheKeyValueMap>,
        field: &str,
        value: DynamicValue,
    ) -> Result<()> {
        // 写入全局缓存
        let cache_key = self.build_cache_key(field)?;
        cache_guard.insert(cache_key, value.clone());
        self.dirty = true;

        log::debug!("Config field updated: field={}, value={:?}", field, value);
        Ok(())
    }

    /// 获取配置字段（从全局缓存读取）
    pub async fn get_config_field(
        &self,
        cache_guard: &GlobalRwLockReadGuard<'_, CacheKeyValueMap>,
        field: &str,
    ) -> Result<DynamicValue> {
        // 从缓存读取
        let cache_key = self.build_cache_key(field)?;
        let value = cache_guard
            .get(&cache_key)
            .cloned()
            .ok_or(AppError::FieldNotFound)?;

        Ok(value)
    }
}

// ======================== DataSource Trait 实现 ========================
#[async_trait(?Send)]
impl DataSource for ConfigDataSource {
    /// 获取数据源名称（缓存key前缀）
    fn name(&self) -> &'static str {
        CONFIG_CACHE_PREFIX
    }

    /// 刷新间隔：每小时刷新一次
    fn refresh_interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    /// 核心逻辑：刷新数据并写入全局缓存
    async fn refresh_with_cache(
        &mut self,
        _system_api: &'static GlobalMutex<DefaultSystemApi>,
        _cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
    ) -> Result<()> {
        // if self.dirty {
        //     let cache_read_guard = self.system_api.lock().await.get_config_api();
        //     self.save_cache_to_storage(&cache_read_guard).await?;
        // }
        Ok(())
    }
}
