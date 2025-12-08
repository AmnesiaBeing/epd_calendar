// src/kernel/data/sources/config.rs
//! 配置数据源模块
//! 实现配置数据的加载、缓存、默认值注册和访问接口

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::data::types::{DynamicValue, FieldMeta};
use crate::kernel::data::{DataSource, DataSourceCache, DataSourceId};
use crate::kernel::system::api::{ConfigApi, HardwareApi, SystemApi};
use heapless::String as HeaplessString;
use heapless::Vec as HeaplessVec;
use postcard::to_allocvec;
use serde::{Deserialize, Serialize};

/// 配置数据源
pub struct ConfigDataSource {
    /// 配置缓存
    cache: GlobalMutex<DataSourceCache>,
    /// 字段元数据
    fields: GlobalMutex<HeaplessVec<FieldMeta, 16>>,
    /// 底层存储接口
    config_storage: &'static dyn ConfigApi,
    /// 硬件API接口
    hardware_api: &'static dyn HardwareApi,
}

impl ConfigDataSource {
    /// 创建新的配置数据源
    pub fn new(
        config_storage: &'static dyn ConfigApi,
        hardware_api: &'static dyn HardwareApi,
    ) -> Self {
        Self {
            cache: GlobalMutex::new(DataSourceCache::default()),
            fields: GlobalMutex::new(HeaplessVec::new()),
            config_storage,
            hardware_api,
        }
    }

    /// 初始化默认配置
    // pub fn init_default_config(&self) -> Result<()> {
    //     // 检查是否已有配置
    //     let existing_config = self.config_storage.read_config()?;

    //     if existing_config.is_some() {
    //         // 已有配置，跳过初始化
    //         return Ok(());
    //     }

    //     // 没有配置，使用默认值初始化
    //     let defaults = self.defaults.lock();
    //     let mut config_entries = HeaplessVec::new();

    //     for default in defaults.iter() {
    //         config_entries.push((default.field.clone(), default.value.clone()))?;
    //     }

    //     // 保存默认配置
    //     let data = to_allocvec(&config_entries)?;
    //     self.config_storage.write_config(&data)?;

    //     log::info!("Default config initialized with {} fields", defaults.len());
    //     Ok(())
    // }

    /// 从存储加载配置
    fn load_config_from_storage(
        &self,
    ) -> Result<HeaplessVec<(HeaplessVec<u8, 32>, DynamicValue), 16>> {
        match self.config_storage.read_config()? {
            Some(data) => {
                // 解析配置数据
                let config = postcard::from_bytes::<
                    HeaplessVec<(HeaplessVec<u8, 32>, DynamicValue), 16>,
                >(&data)?;
                Ok(config)
            }
            None => {
                // 使用默认配置
                let defaults = self.defaults.lock();
                let mut config = HeaplessVec::new();

                for default in defaults.iter() {
                    config.push((default.field.clone(), default.value.clone()))?;
                }

                Ok(config)
            }
        }
    }

    /// 保存配置到存储
    fn save_config_to_storage(
        &self,
        config: &HeaplessVec<(HeaplessVec<u8, 32>, DynamicValue), 16>,
    ) -> Result<()> {
        let data = to_allocvec(config)?;
        self.config_storage.write_config(&data)
    }

    /// 设置配置字段
    pub async fn set_config_field(&mut self, field: &str, value: DynamicValue) -> Result<()> {
        // 加载当前配置
        let mut config = self.load_config_from_storage()?;

        // 更新字段值
        let mut updated = false;
        for entry in &mut config {
            let entry_field =
                core::str::from_utf8(&entry.0).map_err(|_| AppError::InvalidFieldName)?;
            if entry_field == field {
                entry.1 = value.clone();
                updated = true;
                break;
            }
        }

        // 如果字段不存在，添加新字段
        if !updated {
            let mut field_vec = HeaplessVec::new();
            field_vec.extend_from_slice(field.as_bytes())?;
            config.push((field_vec, value.clone()))?;
        }

        // 保存到存储
        self.save_config_to_storage(&config)?;

        // 直接更新缓存，避免调用refresh方法
        let field_name = HeaplessString::try_from(field)?;
        let mut cache = self.cache.lock();
        cache.await.set_field(field_name, value.clone())?;
        cache
            .await
            .mark_valid(self.hardware_api.get_system_timestamp());

        log::debug!("Config field updated: field={}, value={:?}", field, value);
        Ok(())
    }

    /// 获取配置字段
    pub async fn get_config_field(&self, field: &str) -> Result<DynamicValue> {
        // 从缓存获取
        let cache = self.cache.lock();
        cache
            .await
            .get_field(field)
            .cloned()
            .ok_or(AppError::FieldNotFound)
    }

    /// 获取字段列表的副本
    pub async fn get_fields(&self) -> HeaplessVec<FieldMeta, 16> {
        let fields = self.fields.lock();
        fields.await.clone()
    }
}

impl DataSource for ConfigDataSource {
    /// 获取数据源ID
    fn id(&self) -> DataSourceId {
        DataSourceId::Config
    }

    /// 获取数据源名称
    fn name(&self) -> &'static str {
        "config"
    }

    /// 获取数据源字段列表
    fn fields(&self) -> &[FieldMeta] {
        unimplemented!()
    }

    /// 获取字段值
    fn get_field_value(&self, field: &str) -> Result<DynamicValue> {
        self.get_config_field(field)
    }

    /// 刷新数据源
    async fn refresh(&mut self, _system_api: &dyn SystemApi) -> Result<()> {
        // 加载配置
        let config = self.load_config_from_storage()?;

        // 清空现有缓存
        let mut cache = self.cache.lock();
        cache.await.clear();

        // 更新缓存
        for (field_bytes, value) in config {
            let field =
                core::str::from_utf8(&field_bytes).map_err(|_| AppError::InvalidFieldName)?;
            let field_name =
                HeaplessString::try_from(field).map_err(|_| AppError::InvalidFieldName)?;
            cache.await.set_field(field_name, value.clone())?;
        }

        // 标记缓存为有效
        cache
            .await
            .mark_valid(self.hardware_api.get_system_timestamp());

        log::debug!("Config data refreshed");
        Ok(())
    }

    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> u32 {
        // 配置数据源不需要定期刷新，返回一个很大的值
        3600
    }

    /// 检查数据是否有效
    fn is_data_valid(&self) -> bool {
        let cache = self.cache.lock();
        cache.valid
    }

    /// 获取缓存
    fn get_cache(&self) -> &DataSourceCache {
        // 注意：在实际项目中，应该重新设计DataSource trait的get_cache方法
        // 当前实现为了满足编译要求，返回一个静态的空缓存
        static EMPTY_CACHE: DataSourceCache = DataSourceCache::default();
        &EMPTY_CACHE
    }

    /// 获取可变缓存
    fn get_cache_mut(&mut self) -> &mut DataSourceCache {
        // 注意：在实际项目中，应该重新设计DataSource trait的get_cache_mut方法
        // 当前实现为了满足编译要求，返回一个静态的空缓存
        static mut EMPTY_CACHE: DataSourceCache = DataSourceCache::default();
        unsafe { &mut EMPTY_CACHE }
    }
}
