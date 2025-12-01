// src/driver/storage/linux.rs
use crate::common::error::{AppError, Result};
use crate::driver::storage::ConfigStorage;
use std::fs;
use std::path::Path;

/// Linux 模拟器配置存储实现（使用文件）
pub struct FileConfigStorage {
    file_path: String,
    config_size: usize,
}

impl FileConfigStorage {
    pub fn new(file_path: &str, config_size: usize) -> Result<Self> {
        Ok(Self {
            file_path: file_path.to_string(),
            config_size,
        })
    }
}

impl ConfigStorage for FileConfigStorage {
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

    fn write_config_block(&mut self, data: &[u8]) -> Result<()> {
        // 确保数据大小正确
        let mut buffer = vec![0xFF; self.config_size];
        let copy_len = data.len().min(self.config_size);
        buffer[..copy_len].copy_from_slice(&data[..copy_len]);

        fs::write(&self.file_path, &buffer).map_err(|_| AppError::StorageError)?;

        Ok(())
    }

    fn erase_config(&mut self) -> Result<()> {
        // 模拟擦除：写入全 0xFF
        let buffer = vec![0xFF; self.config_size];
        fs::write(&self.file_path, &buffer).map_err(|_| AppError::StorageError)?;
        Ok(())
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultConfigStorage = FileConfigStorage;
