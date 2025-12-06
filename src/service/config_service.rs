// src/service/config_manager.rs
// src/service/config_service.rs

//! 配置服务模块 - 处理系统配置的序列化、存储和验证
//! 
//! 该模块提供配置管理功能，包括配置的加载、保存、验证和版本迁移。

use crate::common::config::{CONFIG_MAGIC, MAX_CONFIG_SIZE, SystemConfig, default_config_version};
use crate::common::error::{AppError, Result};
use crate::driver::storage::{ConfigStorage, DefaultConfigStorage};
use alloc::vec::Vec;
use postcard::{from_bytes, to_allocvec};

/// 配置管理器，处理配置的序列化、存储和验证
pub struct ConfigService {
    /// 存储驱动实例
    storage: DefaultConfigStorage,
    /// 当前内存中的配置数据
    current_config: SystemConfig,
    /// 配置是否已修改但未保存
    config_dirty: bool,
}

impl ConfigService {
    /// 创建新的配置服务实例
    /// 
    /// # 参数
    /// - `storage`: 存储驱动实例
    /// 
    /// # 返回值
    /// 返回新的ConfigService实例
    pub fn new(storage: DefaultConfigStorage) -> Self {
        Self {
            storage,
            current_config: SystemConfig::default(),
            config_dirty: false,
        }
    }

    /// 从存储中加载配置
    /// 
    /// # 返回值
    /// - `Result<()>`: 加载成功返回Ok(()), 失败返回错误
    pub fn load_config(&mut self) -> Result<()> {
        match self.storage.read_config_block()? {
            Some(data) => {
                // 验证数据有效性
                self.validate_and_load_config(&data)
            }
            None => {
                // 没有存储的配置，使用默认值
                log::info!("No stored config found, using defaults");
                Ok(())
            }
        }
    }

    /// 验证并加载配置数据
    /// 
    /// # 参数
    /// - `data`: 配置数据字节切片
    /// 
    /// # 返回值
    /// - `Result<()>`: 验证和加载成功返回Ok(()), 失败返回错误
    fn validate_and_load_config(&mut self, data: &[u8]) -> Result<()> {
        if data.len() < 8 {
            return Err(AppError::ConfigInvalid);
        }

        // 检查魔法数字
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != CONFIG_MAGIC {
            log::warn!(
                "Invalid config magic: 0x{:08X}, expected 0x{:08X}",
                magic,
                CONFIG_MAGIC
            );
            return Err(AppError::ConfigInvalid);
        }

        // 检查数据大小
        let data_len = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        if data_len + 8 != data.len() {
            log::warn!(
                "Config size mismatch: expected {}, got {}",
                data_len + 8,
                data.len()
            );
            return Err(AppError::ConfigInvalid);
        }

        // 反序列化配置
        self.current_config = from_bytes(&data[8..]).map_err(|e| {
            log::error!("Failed to deserialize config: {:?}", e);
            AppError::ConfigDeserializationError
        })?;

        log::info!(
            "Config loaded successfully, version: {}",
            self.current_config.config_version
        );
        Ok(())
    }

    /// 保存当前配置到存储
    /// 
    /// # 返回值
    /// - `Result<()>`: 保存成功返回Ok(()), 失败返回错误
    pub fn save_config(&mut self) -> Result<()> {
        // 序列化配置
        let serialized =
            to_allocvec(&self.current_config).map_err(|_| AppError::ConfigSerializationError)?;

        // 检查大小
        if serialized.len() + 8 > MAX_CONFIG_SIZE {
            return Err(AppError::ConfigTooLarge);
        }

        // 构建完整的数据块：魔法数字 + 数据长度 + 序列化数据
        let mut data = Vec::with_capacity(serialized.len() + 8);
        data.extend_from_slice(&CONFIG_MAGIC.to_le_bytes());
        data.extend_from_slice(&(serialized.len() as u32).to_le_bytes());
        data.extend_from_slice(&serialized);

        // 写入存储
        self.storage.write_config_block(&data)?;
        self.config_dirty = false;

        log::info!("Config saved successfully, size: {} bytes", data.len());
        Ok(())
    }

    /// 获取当前配置的引用
    /// 
    /// # 返回值
    /// - `&SystemConfig`: 当前配置的不可变引用
    pub fn get_config(&self) -> &SystemConfig {
        &self.current_config
    }

    /// 获取当前配置的可变引用，标记为脏
    /// 
    /// # 返回值
    /// - `&mut SystemConfig`: 当前配置的可变引用
    pub fn get_config_mut(&mut self) -> &mut SystemConfig {
        self.config_dirty = true;
        &mut self.current_config
    }

    /// 检查配置是否已修改但未保存
    /// 
    /// # 返回值
    /// - `bool`: true表示配置已修改但未保存
    pub fn is_dirty(&self) -> bool {
        self.config_dirty
    }

    /// 重置为默认配置
    pub fn reset_to_default(&mut self) {
        self.current_config = SystemConfig::default();
        self.config_dirty = true;
        log::info!("Config reset to defaults");
    }

    /// 检查配置是否需要升级
    /// 
    /// # 返回值
    /// - `bool`: true表示配置需要版本升级
    pub fn needs_migration(&self) -> bool {
        self.current_config.config_version < default_config_version()
    }

    /// 升级配置版本（如果需要）
    /// 
    /// # 返回值
    /// - `Result<()>`: 升级成功返回Ok(()), 失败返回错误
    pub fn migrate_config(&mut self) -> Result<()> {
        if !self.needs_migration() {
            return Ok(());
        }

        log::info!(
            "Migrating config from version {} to {}",
            self.current_config.config_version,
            default_config_version()
        );

        // 这里可以添加版本迁移逻辑
        // 例如，当从版本1升级到版本2时，可以初始化新增字段

        self.current_config.config_version = default_config_version();
        self.config_dirty = true;

        Ok(())
    }
}