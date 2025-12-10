// src/driver/storage/esp.rs

//! ESP32平台存储驱动实现
//!
//! 提供ESP32平台的Flash存储功能，基于esp-bootloader-esp-idf库实现

use embedded_storage::nor_flash::NorFlash;
use embedded_storage::nor_flash::ReadNorFlash;
use esp_bootloader_esp_idf::partitions::{self, FlashRegion};
use esp_bootloader_esp_idf::partitions::{DataPartitionSubType, PartitionType};
use esp_hal::peripherals::Peripherals;
use esp_storage::FlashStorage;
use static_cell::StaticCell;

use crate::common::error::{AppError, Result};
use crate::kernel::driver::storage::ConfigStorage;

/// Flash存储静态实例
static FLASH_STORAGE: StaticCell<FlashStorage<'static>> = StaticCell::new();
/// 分区表内存缓冲区
static PT_MEM: StaticCell<[u8; partitions::PARTITION_TABLE_MAX_LEN]> = StaticCell::new();

/// 配置数据块大小（4KB）
const CONFIG_SIZE: usize = 4096;

/// ESP32配置存储结构体
///
/// 管理ESP32平台的Flash存储，使用NVS分区进行配置数据持久化
pub struct EspConfigStorageDriver {
    /// Flash存储区域
    flash_storage: FlashRegion<'static, FlashStorage<'static>>,
    /// 配置数据起始地址
    config_address: u32,
}

impl EspConfigStorageDriver {
    /// 创建新的ESP32配置存储实例
    ///
    /// # 参数
    /// - `peripherals`: ESP32硬件外设
    ///
    /// # 返回值
    /// - `Result<Self>`: 存储实例或错误
    pub fn new(peripherals: &Peripherals) -> Result<Self> {
        // 1. 初始化 FlashStorage 并获取静态引用
        let flash_storage_ref = FLASH_STORAGE.init(FlashStorage::new(unsafe {
            peripherals.FLASH.clone_unchecked()
        }));

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
            flash_storage: config_partition,
            config_address: 0,
        })
    }
}

impl ConfigStorage for EspConfigStorageDriver {
    /// 读取配置数据块
    ///
    /// # 返回值
    /// - `Result<Option<alloc::vec::Vec<u8>>>`: 配置数据或None（未初始化）
    fn read_config_block(&mut self) -> Result<Option<alloc::vec::Vec<u8>>> {
        // 创建配置大小的缓冲区
        let mut buffer = alloc::vec![0u8; CONFIG_SIZE];

        // 从Flash读取数据
        if let Err(_) = self
            .flash_storage
            .read(self.config_address, &mut buffer[..])
        {
            return Err(AppError::StorageError);
        }

        // 检查数据是否全为0xFF（未初始化或已擦除）
        let all_erased = buffer.iter().all(|&b| b == 0xFF);
        if all_erased {
            Ok(None)
        } else {
            Ok(Some(buffer))
        }
    }

    /// 写入配置数据块
    ///
    /// # 参数
    /// - `data`: 要写入的配置数据
    ///
    /// # 返回值
    /// - `Result<()>`: 写入结果
    fn write_config_block(&mut self, data: &[u8]) -> Result<()> {
        // 检查输入数据是否为空
        if data.is_empty() {
            return Err(AppError::StorageError);
        }

        // 确保数据不会超出预定义的配置大小
        let write_size = data.len().min(CONFIG_SIZE);

        // 创建写入缓冲区
        let mut buffer = alloc::vec![0u8; write_size];
        buffer.copy_from_slice(&data[..write_size]);

        // 写入数据到Flash
        if let Err(_) = self.flash_storage.write(self.config_address, &buffer[..]) {
            return Err(AppError::StorageError);
        }

        Ok(())
    }

    /// 擦除配置存储区域
    ///
    /// # 返回值
    /// - `Result<()>`: 擦除结果
    fn erase_config(&mut self) -> Result<()> {
        // 创建全0xFF的缓冲区（表示擦除状态）
        let buffer = alloc::vec![0xFF; CONFIG_SIZE];

        // 写入全0xFF来模拟擦除
        if let Err(_) = self.flash_storage.write(self.config_address, &buffer[..]) {
            return Err(AppError::StorageError);
        }

        Ok(())
    }
}
