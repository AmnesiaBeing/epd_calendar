// src/driver/storage/esp.rs
use esp_bootloader_esp_idf::partitions::{self, FlashRegion};
use esp_bootloader_esp_idf::partitions::{DataPartitionSubType, PartitionType};
use esp_hal::peripherals::FLASH;
use esp_storage::FlashStorage;
use static_cell::StaticCell;

use crate::common::error::{AppError, Result};
use crate::driver::storage::ConfigStorage;

static FLASH_STORAGE: StaticCell<FlashStorage<'static>> = StaticCell::new();
static PT_MEM: StaticCell<[u8; partitions::PARTITION_TABLE_MAX_LEN]> = StaticCell::new();

/// ESP32 配置存储实现
pub struct EspConfigStorage {
    flash: FlashRegion<'static, FlashStorage<'static>>,
}

impl EspConfigStorage {
    pub fn new(flash: FLASH<'static>) -> Result<Self> {
        // 1. 初始化 FlashStorage 并获取静态引用
        let flash_storage_ref = FLASH_STORAGE.init(FlashStorage::new(flash));

        // 2. 初始化分区表缓冲区并获取静态引用
        let pt_mem_ref = PT_MEM.init([0u8; partitions::PARTITION_TABLE_MAX_LEN]);

        // 3. 现在可以使用这些 'static 引用来读取分区表
        let pt = partitions::read_partition_table(flash_storage_ref, pt_mem_ref)
            .map_err(|_| AppError::StorageError)?;

        // 4. 查找 NVS 分区
        let config_partition = pt
            .find_partition(PartitionType::Data(DataPartitionSubType::Nvs))
            .map_err(|_| AppError::StorageError)?
            .ok_or(AppError::StorageError)?
            .as_embedded_storage(flash_storage_ref);

        Ok(Self {
            flash: config_partition,
        })
    }
}

impl ConfigStorage for EspConfigStorage {
    fn read_config_block(&mut self) -> Result<Option<alloc::vec::Vec<u8>>> {
        todo!()
    }

    fn write_config_block(&mut self, data: &[u8]) -> Result<()> {
        todo!()
    }

    fn erase_config(&mut self) -> Result<()> {
        todo!()
    }
}

#[cfg(feature = "embedded_esp")]
pub type DefaultConfigStorage = EspConfigStorage;
