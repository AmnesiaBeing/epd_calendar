// src/driver/storage/linux.rs

//! Linux平台存储驱动实现
//! 
//! 提供Linux平台的文件存储功能，使用文件系统进行配置数据持久化

use crate::common::error::{AppError, Result};
use crate::driver::storage::ConfigStorage;
use std::fs;
use std::path::Path;

/// Linux模拟器配置存储结构体
/// 
/// 管理Linux平台的文件存储，使用文件系统进行配置数据持久化
pub struct FileConfigStorage {
    /// 配置文件路径
    file_path: String,
    /// 配置数据块大小
    config_size: usize,
}

impl FileConfigStorage {
    /// 创建新的Linux配置存储实例
    /// 
    /// # 参数
    /// - `file_path`: 配置文件路径
    /// - `config_size`: 配置数据块大小
    /// 
    /// # 返回值
    /// - `Result<Self>`: 存储实例或错误
    pub fn new(file_path: &str, config_size: usize) -> Result<Self> {
        Ok(Self {
            file_path: file_path.to_string(),
            config_size,
        })
    }
}

impl ConfigStorage for FileConfigStorage {
    /// 读取配置数据块
    /// 
    /// # 返回值
    /// - `Result<Option<Vec<u8>>>`: 配置数据或None（文件不存在）
    fn read_config_block(&mut self) -> Result<Option<Vec<u8>>> {
        let path = Path::new(&self.file_path);

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(path).map_err(|_| AppError::StorageError)?;

        if data.len() != self.config_size {
            log::warn!(
                "Config file size mismatch: expected {}, got {}",
                self.config_size,
                data.len()
            );
        }

        Ok(Some(data))
    }

    /// 写入配置数据块
    /// 
    /// # 参数
    /// - `data`: 要写入的配置数据
    /// 
    /// # 返回值
    /// - `Result<()>`: 写入结果
    fn write_config_block(&mut self, data: &[u8]) -> Result<()> {
        // 确保数据大小正确
        let mut buffer = vec![0xFF; self.config_size];
        let copy_len = data.len().min(self.config_size);
        buffer[..copy_len].copy_from_slice(&data[..copy_len]);

        fs::write(&self.file_path, &buffer).map_err(|_| AppError::StorageError)?;

        Ok(())
    }

    /// 擦除配置存储区域
    /// 
    /// # 返回值
    /// - `Result<()>`: 擦除结果
    fn erase_config(&mut self) -> Result<()> {
        // 模拟擦除：写入全 0xFF
        let buffer = vec![0xFF; self.config_size];
        fs::write(&self.file_path, &buffer).map_err(|_| AppError::StorageError)?;
        Ok(())
    }
}

/// Linux平台默认配置存储类型别名
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultConfigStorage = FileConfigStorage;