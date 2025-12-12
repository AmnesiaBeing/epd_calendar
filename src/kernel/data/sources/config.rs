//! 配置数据源模块
//! 实现配置数据的加载、缓存、默认值注册和访问接口

use alloc::boxed::Box;
use async_trait::async_trait;
use core::str::FromStr;
use embassy_time::Duration;
use heapless::{String as HeaplessString, Vec as HeaplessVec, format};

use crate::common::error::{AppError, Result};
use crate::common::{GlobalMutex, GlobalRwLockReadGuard, GlobalRwLockWriteGuard};
use crate::kernel::data::DataSource;
use crate::kernel::data::DynamicValue;
use crate::kernel::data::types::{CacheKeyValueMap, HeaplessString as CacheString, KEY_LENGTH};
use crate::kernel::system::api::{ConfigApi, DefaultSystemApi, SystemApi};

// ======================== 常量定义 ========================
/// 配置缓存key前缀
const CONFIG_CACHE_PREFIX: &str = "config";
/// 最大配置项数量（与静态默认配置匹配）
const MAX_CONFIG_ITEMS: usize = 10;

// ======================== 静态默认配置 ========================
/// 静态默认配置项（编译期确定，无运行时内存分配）
const DEFAULT_CONFIG_ITEMS: &[(&str, DynamicValue)] = &[
    // 网络配置
    ("wifi_ssid", DynamicValue::String(CacheString::new())),
    ("wifi_password", DynamicValue::String(CacheString::new())),
    // 时间配置（东8区）
    ("timezone", DynamicValue::Integer(8)),
    (
        "ntp_server",
        DynamicValue::String(CacheString::try_from("pool.ntp.org").unwrap()),
    ),
    // 天气配置
    ("weather.api_key", DynamicValue::String(CacheString::new())),
    (
        "weather.location_id",
        DynamicValue::String(CacheString::new()),
    ),
];

// ======================== 类型别名（增强可读性） ========================
/// 配置项类型（字段名+值）
type ConfigItem = (HeaplessString<KEY_LENGTH>, DynamicValue);
/// 配置项列表（固定长度，嵌入式友好）
type ConfigItemList = HeaplessVec<ConfigItem, MAX_CONFIG_ITEMS>;

// ======================== 配置数据源结构体（精简，移除defaults） ========================
pub struct ConfigDataSource {
    /// 系统API实例（全局互斥锁保护）
    system_api: &'static GlobalMutex<DefaultSystemApi>,
    /// 配置是否被修改（脏标记）
    dirty: bool,
    /// 标记是否已初始化默认配置到存储（避免重复写入）
    default_initialized: bool,
}

impl ConfigDataSource {
    /// 创建新的配置数据源实例
    pub async fn new(system_api: &'static GlobalMutex<DefaultSystemApi>) -> Result<Self> {
        Ok(Self {
            system_api,
            dirty: false,
            default_initialized: false, // 初始未初始化默认配置
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

    /// 将静态默认配置转换为HeaplessVec（嵌入式友好）
    fn get_default_config_list(&self) -> Result<ConfigItemList> {
        let mut defaults = ConfigItemList::new();
        for (field, value) in DEFAULT_CONFIG_ITEMS {
            defaults
                .push((self.str_to_heapless(field)?, value.clone()))
                .map_err(|_| {
                    log::error!("Failed to push default config item: {}", field);
                    AppError::ConfigInitFailed
                })?;
        }
        Ok(defaults)
    }

    // ======================== 存储交互逻辑（核心重构） ========================
    /// 初始化默认配置到存储（仅当存储为空且未初始化过时执行）
    async fn init_default_to_storage(&mut self) -> Result<()> {
        // 已初始化则直接返回
        if self.default_initialized {
            return Ok(());
        }

        // 检查存储是否已有数据
        let system_api_guard = self.system_api.lock().await;
        let config_api = system_api_guard.get_config_api();
        let existing_data = config_api.read_config().await?;
        drop(system_api_guard);

        // 存储已有数据，标记初始化完成并返回
        if existing_data.is_some() {
            self.default_initialized = true;
            log::debug!("Config storage has data, skip default init");
            return Ok(());
        }

        // 存储无数据，写入默认配置
        let default_config = self.get_default_config_list()?;
        // 序列化默认配置
        let data = postcard::to_allocvec(&default_config).map_err(|_| {
            log::error!("Failed to serialize default config");
            AppError::InvalidConfigData
        })?;

        // 写入存储
        let system_api_guard = self.system_api.lock().await;
        let config_api = system_api_guard.get_config_api();
        config_api.write_config(&data).await?;
        drop(system_api_guard);

        self.default_initialized = true;
        log::info!("Default config initialized to storage");
        Ok(())
    }

    /// 从存储加载配置（优先存储，无则自动初始化默认配置）
    async fn load_config_from_storage(&mut self) -> Result<ConfigItemList> {
        // 确保默认配置已初始化（首次加载时执行）
        self.init_default_to_storage().await?;

        // 读取存储数据
        let config_data = {
            let system_api_guard = self.system_api.lock().await;
            let config_api = system_api_guard.get_config_api();
            config_api.read_config().await?
        };

        // 解析配置（存储已有数据，必能解析）
        let config = match config_data {
            Some(data) => postcard::from_bytes::<ConfigItemList>(&data).map_err(|_| {
                log::error!("Failed to parse config from storage");
                AppError::InvalidConfigData
            })?,
            None => {
                // 理论上不会走到这里（已初始化默认配置），兜底返回默认配置
                log::warn!("Config storage empty after init, use default");
                self.get_default_config_list()?
            }
        };

        Ok(config)
    }

    /// 保存配置到存储（从全局缓存读取）
    async fn save_config_to_storage(
        &mut self,
        cache_guard: &GlobalRwLockReadGuard<'_, CacheKeyValueMap>,
    ) -> Result<()> {
        // 从缓存构建配置列表（仅包含默认配置的字段）
        let mut config = ConfigItemList::new();
        for (field, _) in DEFAULT_CONFIG_ITEMS {
            let cache_key = self.build_cache_key(field)?;
            let value = cache_guard.get(&cache_key).cloned().unwrap_or_else(|| {
                // 兜底：使用空字符串
                DynamicValue::String(HeaplessString::new())
            });
            config
                .push((self.str_to_heapless(field)?, value))
                .map_err(|_| {
                    log::error!("Failed to push config item for storage: {}", field);
                    AppError::InvalidConfigData
                })?;
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
        // 校验字段是否在默认配置列表中（防止非法字段）
        if !DEFAULT_CONFIG_ITEMS.iter().any(|(f, _)| f == &field) {
            log::error!("Invalid config field: {}", field);
            return Err(AppError::InvalidFieldName);
        }

        // 写入全局缓存
        let cache_key = self.build_cache_key(field)?;
        cache_guard.insert(cache_key, value.clone());
        self.dirty = true;

        log::debug!("Config field updated: field={}, value={:?}", field, value);
        Ok(())
    }

    /// 获取配置字段（从全局缓存读取，无则返回默认值）
    pub async fn get_config_field(
        &self,
        cache_guard: &GlobalRwLockReadGuard<'_, CacheKeyValueMap>,
        field: &str,
    ) -> Result<DynamicValue> {
        // 校验字段合法性
        if !DEFAULT_CONFIG_ITEMS.iter().any(|(f, _)| f == &field) {
            log::error!("Invalid config field: {}", field);
            return Err(AppError::InvalidFieldName);
        }

        // 从缓存读取，无则返回静态默认值
        let cache_key = self.build_cache_key(field)?;
        let value = cache_guard
            .get(&cache_key)
            .cloned()
            .or_else(|| {
                DEFAULT_CONFIG_ITEMS
                    .iter()
                    .find(|(f, _)| f == &field)
                    .map(|(_, v)| v.clone())
            })
            .ok_or(AppError::FieldNotFound)?;

        Ok(value)
    }

    /// 保存缓存到存储（仅脏标记为true时）
    async fn save_cache_to_storage(
        &mut self,
        cache_guard: &GlobalRwLockReadGuard<'_, CacheKeyValueMap>,
    ) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.save_config_to_storage(cache_guard).await
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
        cache_guard: &mut GlobalRwLockWriteGuard<CacheKeyValueMap>,
    ) -> Result<()> {
        // 步骤1：保存脏数据到存储（需获取缓存读守卫）
        // 注意：此处假设调度器提供全局缓存的读守卫获取方式

        // 步骤2：从存储加载配置（自动初始化默认配置）
        let config = self.load_config_from_storage().await?;

        // 步骤4：写入存储配置到缓存
        let mut updated_fields = 0;
        for (field_name, value) in config {
            let cache_key = self.build_cache_key(field_name.as_str())?;
            cache_guard.insert(cache_key, value);
            updated_fields += 1;
        }

        // 步骤5：补全默认配置（防止存储中缺失部分字段）
        for (field, default_value) in DEFAULT_CONFIG_ITEMS {
            let cache_key = self.build_cache_key(field)?;
            if !cache_guard.contains_key(&cache_key) {
                cache_guard.insert(cache_key, default_value.clone());
                updated_fields += 1;
            }
        }

        log::debug!("Config refreshed, {} fields updated", updated_fields);
        Ok(())
    }
}
