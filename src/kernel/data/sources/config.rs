// src/kernel/data/sources/config.rs
//! 配置数据源模块
//! 实现配置数据的加载、缓存、默认值注册和访问接口

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::data::types::{DynamicValue, FieldMeta};
use crate::kernel::data::{DataSource, DataSourceCache};
use crate::kernel::system::api::{ConfigApi, DefaultSystemApi, SystemApi};

use alloc::boxed::Box;
use async_trait::async_trait;
use core::str::FromStr;
use embassy_time::{Duration, Instant};
use heapless::{String, Vec};

type String32 = String<32>;

/// 默认配置项定义
#[derive(Debug, Clone)]
pub struct DefaultConfigItem {
    /// 配置字段名称
    field: String32,
    /// 默认值
    value: DynamicValue,
}

/// 配置数据源
pub struct ConfigDataSource {
    /// 配置缓存
    cache: DataSourceCache,
    /// 字段元数据
    fields: Vec<FieldMeta, 32>,
    /// 系统API接口
    system_api: &'static GlobalMutex<DefaultSystemApi>,
    /// 配置是否被修改
    dirty: bool,
    /// 默认配置项列表
    defaults: Vec<DefaultConfigItem, 16>,
}

impl ConfigDataSource {
    /// 创建新的配置数据源
    pub async fn new(system_api: &'static GlobalMutex<DefaultSystemApi>) -> Result<Self> {
        let mut instance = Self {
            cache: DataSourceCache::default(),
            fields: Vec::new(),
            system_api,
            dirty: false,
            defaults: Vec::new(),
        };

        // 初始化默认配置项
        instance.init_defaults();

        Ok(instance)
    }

    /// 初始化默认配置项
    fn init_defaults(&mut self) {
        // 网络配置
        self.defaults
            .push(DefaultConfigItem {
                field: String::from_str("wifi_ssid").unwrap(),
                value: DynamicValue::String(String::try_from("").unwrap()),
            })
            .unwrap();

        self.defaults
            .push(DefaultConfigItem {
                field: String::from_str("wifi_password").unwrap(),
                value: DynamicValue::String(String::try_from("").unwrap()),
            })
            .unwrap();

        // 时间配置
        self.defaults
            .push(DefaultConfigItem {
                field: String::from_str("timezone").unwrap(),
                value: DynamicValue::Integer(-8),
            })
            .unwrap();

        self.defaults
            .push(DefaultConfigItem {
                field: String::from_str("ntp_server").unwrap(),
                value: DynamicValue::String(String::try_from("pool.ntp.org").unwrap()),
            })
            .unwrap();

        // 天气配置
        self.defaults
            .push(DefaultConfigItem {
                field: String::from_str("weather_api_key").unwrap(),
                value: DynamicValue::String(String::try_from("").unwrap()),
            })
            .unwrap();

        self.defaults
            .push(DefaultConfigItem {
                field: String::from_str("weather_location").unwrap(),
                value: DynamicValue::String(String::try_from("").unwrap()),
            })
            .unwrap();

        // 显示配置
        self.defaults
            .push(DefaultConfigItem {
                field: String::from_str("display_brightness").unwrap(),
                value: DynamicValue::Integer(100),
            })
            .unwrap();

        // 初始化字段元数据
        for default in self.defaults.iter() {
            self.fields
                .push(FieldMeta {
                    name: default.field.clone(),
                    content: default.value.clone(),
                })
                .unwrap();
        }
    }

    /// 从存储加载配置
    async fn load_config_from_storage(&self) -> Result<Vec<(String32, DynamicValue), 16>> {
        let config_data = {
            let system_api = self.system_api.lock().await;
            let config_api = system_api.get_config_api();
            config_api.read_config().await?
        };

        match config_data {
            Some(data) => {
                // 解析配置数据
                let config = postcard::from_bytes::<Vec<(String32, DynamicValue), 16>>(&data)
                    .map_err(|_| AppError::InvalidConfigData)?;
                Ok(config)
            }
            None => {
                // 使用默认配置
                let mut config = Vec::new();

                for default in self.defaults.iter() {
                    config
                        .push((default.field.clone(), default.value.clone()))
                        .map_err(|_| AppError::InvalidConfigData)?;
                }

                Ok(config)
            }
        }
    }

    /// 保存配置到存储
    async fn save_config_to_storage(
        &mut self,
        config: &Vec<(String32, DynamicValue), 16>,
    ) -> Result<()> {
        let data = postcard::to_allocvec(config).map_err(|_| AppError::InvalidConfigData)?;

        let system_api = self.system_api.lock().await;
        let config_api = system_api.get_config_api();
        config_api.write_config(&data).await?;

        // 保存成功后清除脏标记
        self.dirty = false;

        log::debug!("Config saved to storage with {} fields", config.len());
        Ok(())
    }

    /// 设置配置字段
    pub async fn set_config_field(&mut self, field: &str, value: DynamicValue) -> Result<()> {
        // 更新缓存
        let field_name = String::try_from(field).map_err(|_| AppError::InvalidFieldName)?;

        self.cache.set_field(field_name.clone(), value.clone())?;
        self.cache.mark_valid(Instant::now());

        // 设置脏标记
        self.dirty = true;

        log::debug!("Config field updated: field={}, value={:?}", field, value);
        Ok(())
    }

    /// 获取配置字段
    pub async fn get_config_field(&self, field: &str) -> Result<DynamicValue> {
        // 从缓存获取
        self.cache
            .get_field(field)
            .cloned()
            .ok_or(AppError::FieldNotFound)
    }

    /// 将缓存中的配置保存到存储
    async fn save_cache_to_storage(&mut self) -> Result<()> {
        // 检查是否有修改
        if !self.dirty {
            return Ok(()); // 没有修改，不需要保存
        }

        // 从缓存构建配置
        let mut config = Vec::new();

        for (cache_field_name, _) in &self.cache.fields {
            if let Some(value) = self.cache.get_field(&cache_field_name) {
                config
                    .push((cache_field_name.clone(), value.clone()))
                    .map_err(|_| AppError::InvalidConfigData)?;
            }
        }

        // 保存到存储
        self.save_config_to_storage(&config).await
    }
}

#[async_trait(?Send)]
impl DataSource for ConfigDataSource {
    /// 获取数据源名称
    fn name(&self) -> &'static str {
        "config"
    }

    /// 获取字段值
    fn get_field_value(&self, field: &str) -> Result<DynamicValue> {
        // 从缓存获取
        self.cache
            .get_field(field)
            .cloned()
            .ok_or(AppError::FieldNotFound)
    }

    /// 刷新数据源
    async fn refresh(&mut self, _system_api: &'static GlobalMutex<DefaultSystemApi>) -> Result<()> {
        // 如果配置被修改，先保存到存储
        if let Err(e) = self.save_cache_to_storage().await {
            log::warn!("Failed to save config to storage: {:?}", e);
        }

        // 从存储加载最新配置
        let config = self.load_config_from_storage().await?;

        // 更新缓存
        self.cache.clear();

        let mut updated_fields = 0;
        for (field_name, value) in config {
            if self.cache.set_field(field_name, value).is_ok() {
                updated_fields += 1;
            }
        }

        // 标记缓存为有效
        self.cache.mark_valid(Instant::now());

        // 确保所有默认字段都在缓存中
        for default in self.defaults.iter() {
            let field_str = default.field.as_str();
            if self.cache.get_field(field_str).is_none() {
                self.cache
                    .set_field(default.field.clone(), default.value.clone())
                    .unwrap();
                updated_fields += 1;
            }
        }

        log::debug!("Config data refreshed, {} fields updated", updated_fields);
        Ok(())
    }

    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> Duration {
        // 配置数据源不需要频繁刷新，每小时刷新一次
        Duration::from_secs(3600)
    }
}
