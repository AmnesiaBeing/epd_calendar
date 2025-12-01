// src/driver/storage/mod.rs

use alloc::vec::Vec;

use crate::common::error::Result;

/// 简化的存储驱动 trait，专注于配置存储
pub trait ConfigStorage {
    /// 读取整个配置数据块
    fn read_config_block(&mut self) -> Result<Option<Vec<u8>>>;

    /// 写入整个配置数据块
    fn write_config_block(&mut self, data: &[u8]) -> Result<()>;

    /// 擦除配置存储区域
    fn erase_config(&mut self) -> Result<()>;
}

#[cfg(feature = "embedded_esp")]
mod esp;
#[cfg(feature = "embedded_esp")]
pub use esp::EspConfigStorage as DefaultConfigStorage;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
mod linux;
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub use linux::FileConfigStorage as DefaultConfigStorage;
