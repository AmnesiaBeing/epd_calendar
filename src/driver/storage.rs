// src/driver/storage.rs

use embedded_storage_async::nor_flash::NorFlash;
use sequential_storage::map::{Value, fetch_item, store_item};
use sequential_storage::{
    cache::{KeyCacheImpl, NoCache},
    map::Key,
};

use crate::common::error::{AppError, Result};
use crate::common::types::SystemConfig;

/// 存储驱动 trait，专门用于存储和读取系统配置
pub trait StorageDriver {
    /// 保存系统配置
    async fn save_config(&mut self, config: &SystemConfig) -> Result<()>;

    /// 加载系统配置
    async fn load_config(&mut self) -> Result<SystemConfig>;
}

/// 配置键常量
mod config_keys {
    // WiFi 配置
    pub const WIFI_SSID: u8 = 0x01;
    pub const WIFI_PASSWORD: u8 = 0x02;
    pub const WIFI_ENCRYPTION: u8 = 0x03;

    // 显示配置
    pub const TIME_FORMAT_24H: u8 = 0x10;
    pub const TEMPERATURE_CELSIUS: u8 = 0x11;
    pub const SHOW_AM_PM: u8 = 0x12;

    // 天气配置
    pub const WEATHER_API_KEY: u8 = 0x20;
    pub const WEATHER_LOCATION: u8 = 0x21;

    // 系统配置
    pub const AUTO_REFRESH_INTERVAL: u8 = 0x30;
    pub const PARTIAL_REFRESH_LIMIT: u8 = 0x31;
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
    data_buffer: Vec<u8>,
    _key: core::marker::PhantomData<K>,
}

impl<F, C, K> KVStorage<F, C, K>
where
    F: NorFlash,
    C: KeyCacheImpl<K>,
    K: Key,
{
    /// 从存储中读取键值对（上层接口）
    pub async fn fetch_kv<'d, V: Value<'d>>(&'d mut self, key: &K) -> Option<V> {
        // 调用底层库的fetch_item，传入Board内部的资源
        fetch_item(
            &mut self.flash,
            self.flash_range.clone(),
            &mut self.key_cache,
            &mut self.data_buffer,
            key,
        )
        .await
        .unwrap_or(None)
    }

    /// 向存储中写入键值对（上层接口）
    pub async fn store_kv<'d, V: Value<'d>>(&mut self, key: &K, value: &V) {
        // 调用底层库的store_item，传入Board内部的资源
        let _ = store_item(
            &mut self.flash,
            self.flash_range.clone(),
            &mut self.key_cache,
            &mut self.data_buffer,
            key,
            value,
        )
        .await;
    }
}

/// 基于你现有 KVStorage 的存储驱动实现
pub struct KvStorageDriver<F>
where
    F: NorFlash,
{
    storage: KVStorage<F, NoCache, u8>,
}

impl<F> KvStorageDriver<F>
where
    F: NorFlash,
{
    pub fn new(storage: KVStorage<F, NoCache, u8>) -> Self {
        Self { storage }
    }
}

impl<F> StorageDriver for KvStorageDriver<F>
where
    F: NorFlash,
{
    async fn save_config(&mut self, config: &SystemConfig) -> Result<()> {
        // 保存 WiFi 配置
        // self.storage
        //     .store_kv(&config_keys::WIFI_SSID, &config.wifi_ssid)
        //     .await;
        // self.storage
        //     .store_kv(&config_keys::WIFI_PASSWORD, &config.wifi_password)
        //     .await;
        // self.storage
        //     .store_kv(
        //         &config_keys::WIFI_ENCRYPTION,
        //         &(config.wifi_encryption as u8),
        //     )
        //     .await;

        // 保存显示配置
        self.storage
            .store_kv(&config_keys::TIME_FORMAT_24H, &config.time_format_24h)
            .await;
        self.storage
            .store_kv(
                &config_keys::TEMPERATURE_CELSIUS,
                &config.temperature_celsius,
            )
            .await;
        self.storage
            .store_kv(&config_keys::SHOW_AM_PM, &config.show_am_pm)
            .await;

        // 保存天气配置
        // self.storage
        //     .store_kv(&config_keys::WEATHER_API_KEY, &config.weather_api_key)
        //     .await;
        // self.storage
        //     .store_kv(&config_keys::WEATHER_LOCATION, &config.weather_location)
        //     .await;

        // 保存系统配置
        self.storage
            .store_kv(
                &config_keys::AUTO_REFRESH_INTERVAL,
                &config.auto_refresh_interval,
            )
            .await;
        self.storage
            .store_kv(
                &config_keys::PARTIAL_REFRESH_LIMIT,
                &config.partial_refresh_limit,
            )
            .await;

        Ok(())
    }

    async fn load_config(&mut self) -> Result<SystemConfig> {
        let mut config = SystemConfig::default();

        // 加载 WiFi 配置
        // if let Some(ssid) = self.storage.fetch_kv(&config_keys::WIFI_SSID).await {
        //     config.wifi_ssid = ssid;
        // }

        // if let Some(password) = self.storage.fetch_kv(&config_keys::WIFI_PASSWORD).await {
        //     config.wifi_password = password;
        // }

        // if let Some(encryption) = self.storage.fetch_kv(&config_keys::WIFI_ENCRYPTION).await {
        //     config.wifi_encryption = match encryption {
        //         0 => WifiEncryption::None,
        //         1 => WifiEncryption::WEP,
        //         2 => WifiEncryption::WPA,
        //         3 => WifiEncryption::WPA2,
        //         4 => WifiEncryption::WPA3,
        //         _ => WifiEncryption::WPA2,
        //     };
        // }

        // 加载显示配置
        if let Some(time_format) = self.storage.fetch_kv(&config_keys::TIME_FORMAT_24H).await {
            config.time_format_24h = time_format;
        }

        if let Some(temp_celsius) = self
            .storage
            .fetch_kv(&config_keys::TEMPERATURE_CELSIUS)
            .await
        {
            config.temperature_celsius = temp_celsius;
        }

        if let Some(show_am_pm) = self.storage.fetch_kv(&config_keys::SHOW_AM_PM).await {
            config.show_am_pm = show_am_pm;
        }

        // 加载天气配置
        // if let Some(api_key) = self.storage.fetch_kv(&config_keys::WEATHER_API_KEY).await {
        //     config.weather_api_key = api_key;
        // }

        // if let Some(location) = self.storage.fetch_kv(&config_keys::WEATHER_LOCATION).await {
        //     config.weather_location = location;
        // }

        // 加载系统配置
        if let Some(refresh_interval) = self
            .storage
            .fetch_kv(&config_keys::AUTO_REFRESH_INTERVAL)
            .await
        {
            config.auto_refresh_interval = refresh_interval;
        }

        if let Some(refresh_limit) = self
            .storage
            .fetch_kv(&config_keys::PARTIAL_REFRESH_LIMIT)
            .await
        {
            config.partial_refresh_limit = refresh_limit;
        }

        Ok(config)
    }
}

// Simulator 和 Linux 存储实现
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embedded_storage_std_async_mock::FlashMock;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultStorageDriver = KvStorageDriver<FlashMock<32, 32, 512>>;

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
