// src/driver/storage/esp.rs
use alloc::vec::{self, Vec};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use esp_hal::peripherals::FLASH;
use esp_storage::FlashStorage;
use partitions::{DataPartitionSubType, PartitionType};

use crate::common::error::{AppError, Result};
use crate::driver::storage::ConfigStorage;

/// ESP32 配置存储实现
pub struct EspConfigStorage {
    flash: FlashStorage,
    config_offset: u32,
    config_size: usize,
}

impl EspConfigStorage {
    pub fn new(flash: FLASH) -> Result<Self> {
        let mut flash_storage = FlashStorage::new(flash);

        // 读取分区表，查找 NVS 分区
        let mut pt_mem = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
        let pt = partitions::read_partition_table(&mut flash_storage, &mut pt_mem)
            .map_err(|_| AppError::StorageError)?;

        // 查找 NVS 分区或创建配置分区
        let config_partition = pt
            .find_partition(PartitionType::Data(DataPartitionSubType::Nvs))
            .or_else(|| {
                // 如果没有 NVS 分区，尝试查找其他数据分区
                pt.find_partition(PartitionType::Data(DataPartitionSubType::Data))
            })
            .ok_or(AppError::StorageError)?
            .ok_or(AppError::StorageError)?;

        let config_offset = config_partition.offset();
        let config_size = config_partition.size() as usize;

        log::info!(
            "Config storage: offset=0x{:08X}, size={} bytes",
            config_offset,
            config_size
        );

        Ok(Self {
            flash: flash_storage,
            config_offset,
            config_size,
        })
    }
}

impl ConfigStorage for EspConfigStorage {
    fn read_config_block(&mut self) -> Result<Option<Vec<u8>>> {
        // 读取整个配置区域
        let mut buffer = vec![0u8; self.config_size];
        self.flash
            .read(self.config_offset, &mut buffer)
            .map_err(|_| AppError::StorageError)?;

        // 检查是否为空（全为 0xFF）
        if buffer.iter().all(|&b| b == 0xFF) {
            return Ok(None);
        }

        Ok(Some(buffer))
    }

    fn write_config_block(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.config_size {
            return Err(AppError::ConfigTooLarge);
        }

        // 擦除整个配置区域
        self.flash
            .erase(
                self.config_offset,
                self.config_offset + self.config_size as u32,
            )
            .map_err(|_| AppError::StorageError)?;

        // 写入新数据
        self.flash
            .write(self.config_offset, data)
            .map_err(|_| AppError::StorageError)?;

        Ok(())
    }

    fn erase_config(&mut self) -> Result<()> {
        self.flash
            .erase(
                self.config_offset,
                self.config_offset + self.config_size as u32,
            )
            .map_err(|_| AppError::StorageError)?;
        Ok(())
    }
}

#[cfg(feature = "embedded_esp")]
pub type DefaultConfigStorage = EspConfigStorage;
