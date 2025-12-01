// src/driver/storage.rs

use embedded_storage::nor_flash::NorFlash;

use crate::common::error::Result;

/// 存储驱动 trait，专门用于存储和读取系统配置
pub trait StorageDriver {
    /// 保存系统配置
    async fn save_config(&mut self, config: &SystemConfig) -> Result<()>;

    /// 加载系统配置
    async fn load_config(&mut self) -> Result<SystemConfig>;
}

pub struct KVStorage<F, C, K>
where
    F: NorFlash,
    C: KeyCacheImpl<K>,
    K: Key,
{
    flash: F,
    flash_range: core::ops::Range<u32>,
    key_cache: C,
    data_buffer: heapless::Vec<u8, 256>,
    _key: core::marker::PhantomData<K>,
}

impl<F, C, K> KVStorage<F, C, K>
where
    F: NorFlash,
    C: KeyCacheImpl<K>,
    K: Key,
{
    /// 从存储中读取键值对（上层接口）
    pub fn fetch_kv<'d, V: Value<'d>>(&'d mut self, key: &K) -> Option<V> {
        // 调用底层库的fetch_item，传入Board内部的资源
        fetch_item(
            &mut self.flash,
            self.flash_range.clone(),
            &mut self.key_cache,
            &mut self.data_buffer,
            key,
        )
        .unwrap_or(None)
    }

    /// 向存储中写入键值对（上层接口）
    pub fn store_kv<'d, V: Value<'d>>(&mut self, key: &K, value: &V) {
        // 调用底层库的store_item，传入Board内部的资源
        let _ = store_item(
            &mut self.flash,
            self.flash_range.clone(),
            &mut self.key_cache,
            &mut self.data_buffer,
            key,
            value,
        )
    }
}

// Simulator 和 Linux 存储实现
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embedded_storage_std_async_mock::FlashMock;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultStorageDriver = KvStorageDriver<FlashMock<32, 32, 512>>;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub async fn create_default_storage() -> Result<DefaultStorageDriver> {
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    {
        // 初始化 Flash（使用文件模拟）
        let flash = FlashMock::<32, 32, 512>::new("flash.bin", 4 * 1024 * 1024)
            .map_err(|_| AppError::StorageError)?;

        // 创建 KV 存储
        let flash_range = 0x0000..0x3000;
        let data_buffer = vec![0; 128];

        let kv_storage = KVStorage {
            flash,
            flash_range,
            key_cache: sequential_storage::cache::NoCache::new(),
            data_buffer,
            _key: core::marker::PhantomData,
        };

        Ok(DefaultStorageDriver::new(kv_storage))
    }
}
