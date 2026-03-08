//! 配置持久化工具
//!
//! 使用 postcard 序列化和 CRC32 校验和
//! 版本检查机制：版本不匹配时返回错误，使用默认配置

use crate::types::error::{StorageError, SystemError};
use crate::SystemResult;
use core::marker::PhantomData;
use embedded_storage_async::nor_flash::{NorFlash, ReadNorFlash};
use postcard;
use serde::{Deserialize, Serialize};

const CONFIG_VERSION: u32 = 1;
const CONFIG_MAGIC: u32 = 0x4C585843; // "LXXC" in little endian
const MAX_CONFIG_SIZE: usize = 1024;
const HEADER_SIZE: usize = 32; // postcard序列化ConfigHeader的大小

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigHeader {
    magic: u32,
    version: u32,
    checksum: u32,
}

/// Flash设备trait (异步版本)
pub trait FlashDevice {
    async fn read(&mut self, offset: u32, buf: &mut [u8]) -> SystemResult<()>;
    async fn write(&mut self, offset: u32, buf: &[u8]) -> SystemResult<()>;
    async fn erase(&mut self, from: u32, to: u32) -> SystemResult<()>;
    fn sector_size(&self) -> u32;
}

/// 为所有实现了 NorFlash 的类型提供 FlashDevice 的 blanket implementation
impl<F> FlashDevice for F
where
    F: NorFlash,
{
    async fn read(&mut self, offset: u32, buf: &mut [u8]) -> SystemResult<()> {
        ReadNorFlash::read(self, offset, buf)
            .await
            .map_err(|_| SystemError::StorageError(StorageError::ReadFailed))
    }

    async fn write(&mut self, offset: u32, buf: &[u8]) -> SystemResult<()> {
        NorFlash::write(self, offset, buf)
            .await
            .map_err(|_| SystemError::StorageError(StorageError::WriteFailed))
    }

    async fn erase(&mut self, from: u32, to: u32) -> SystemResult<()> {
        NorFlash::erase(self, from, to)
            .await
            .map_err(|_| SystemError::StorageError(StorageError::WriteFailed))
    }

    fn sector_size(&self) -> u32 {
        4096
    }
}

/// 配置持久化工具
///
/// 泛型参数 F: 实现 FlashDevice trait 的类型
pub struct ConfigPersistence<F: FlashDevice> {
    flash: F,
    offset: u32,
    _marker: PhantomData<F>,
}

impl<F: FlashDevice> ConfigPersistence<F> {
    /// 创建新的配置持久化实例
    pub fn new(flash: F, offset: u32) -> Self {
        Self {
            flash,
            offset,
            _marker: PhantomData,
        }
    }

    /// 计算配置的 CRC32 校验和
    fn calculate_checksum(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFFFFFF;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    /// 从存储中加载配置
    pub async fn load_config<T>(&mut self) -> SystemResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // 读取配置头
        let mut header_buf = [0u8; HEADER_SIZE];
        self.flash.read(self.offset, &mut header_buf).await?;

        // 反序列化配置头
        let header: ConfigHeader = postcard::from_bytes(&header_buf)
            .map_err(|_| SystemError::StorageError(StorageError::Corrupted))?;

        // 检查 magic number
        if header.magic != CONFIG_MAGIC {
            log::info!("Config magic invalid, using default config");
            return Err(SystemError::StorageError(StorageError::Corrupted));
        }

        // 检查版本
        if header.version != CONFIG_VERSION {
            log::info!(
                "Config version mismatch (stored={}, expected={}), using default config",
                header.version,
                CONFIG_VERSION
            );
            return Err(SystemError::StorageError(StorageError::Corrupted));
        }

        // 读取配置数据
        let mut data_buf = [0u8; MAX_CONFIG_SIZE];
        self.flash.read(self.offset + HEADER_SIZE as u32, &mut data_buf).await?;

        // 验证校验和
        if header.checksum != Self::calculate_checksum(&data_buf) {
            log::warn!("Config checksum mismatch, using default config");
            return Err(SystemError::StorageError(StorageError::Corrupted));
        }

        // 反序列化配置
        postcard::from_bytes(&data_buf)
            .map_err(|_| SystemError::StorageError(StorageError::Corrupted))
    }

    /// 保存配置到存储
    pub async fn save_config<T>(&mut self, config: &T) -> SystemResult<()>
    where
        T: Serialize,
    {
        // 使用固定大小的buffer序列化配置
        let mut buf = [0u8; MAX_CONFIG_SIZE];
        postcard::to_slice(config, &mut buf)
            .map_err(|_| SystemError::StorageError(StorageError::WriteFailed))?;

        // 计算校验和
        let checksum = Self::calculate_checksum(&buf);

        // 构建配置头
        let header = ConfigHeader {
            magic: CONFIG_MAGIC,
            version: CONFIG_VERSION,
            checksum,
        };

        // 序列化配置头
        let mut header_buf = [0u8; HEADER_SIZE];
        postcard::to_slice(&header, &mut header_buf)
            .map_err(|_| SystemError::StorageError(StorageError::WriteFailed))?;

        // 擦除配置区域
        let sector_size = self.flash.sector_size();
        let aligned_offset = (self.offset / sector_size) * sector_size;
        self.flash.erase(aligned_offset, aligned_offset + sector_size).await?;

        // 写入配置头和数据
        self.flash.write(self.offset, &header_buf).await?;
        self.flash.write(self.offset + HEADER_SIZE as u32, &buf).await?;

        log::info!("Config saved successfully");
        Ok(())
    }

    /// 恢复出厂设置（擦除配置）
    pub async fn factory_reset(&mut self) -> SystemResult<()> {
        let sector_size = self.flash.sector_size();
        let aligned_offset = (self.offset / sector_size) * sector_size;
        self.flash.erase(aligned_offset, aligned_offset + sector_size).await?;

        log::info!("Factory reset completed");
        Ok(())
    }

    /// 检查配置是否存在
    pub async fn config_exists(&mut self) -> bool {
        let mut magic_buf = [0u8; 4];
        if self.flash.read(self.offset, &mut magic_buf).await.is_err() {
            return false;
        }

        let magic = u32::from_le_bytes(magic_buf);
        magic == CONFIG_MAGIC
    }
}