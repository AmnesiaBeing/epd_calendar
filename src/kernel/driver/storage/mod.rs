// src/kernel/driver/storage/mod.rs

//! 存储驱动模块
//!
//! 提供配置数据持久化存储的抽象层，支持不同平台的存储实现
//!
//! ## 功能
//! - 定义统一的配置存储接口 `ConfigStorage`
//! - 支持ESP32（Flash存储）和Linux（文件存储）平台
//! - 提供配置数据的读取、写入和擦除功能

use alloc::vec::Vec;

use crate::common::error::Result;

/// 配置存储驱动接口定义
///
/// 提供配置数据的持久化存储功能
pub trait ConfigStorage {
    /// 读取整个配置数据块
    ///
    /// # 返回值
    /// - `Result<Option<Vec<u8>>>`: 配置数据或None（未初始化）
    fn read_config_block(&mut self) -> Result<Option<Vec<u8>>>;

    /// 写入整个配置数据块
    ///
    /// # 参数
    /// - `data`: 要写入的配置数据
    ///
    /// # 返回值
    /// - `Result<()>`: 写入结果
    fn write_config_block(&mut self, data: &[u8]) -> Result<()>;

    /// 擦除配置存储区域
    ///
    /// # 返回值
    /// - `Result<()>`: 擦除结果
    fn erase_config(&mut self) -> Result<()>;
}

// 默认存储驱动选择
#[cfg(feature = "esp32")]
mod esp32;
#[cfg(feature = "esp32")]
pub use esp32::EspConfigStorageDriver as DefaultConfigStorageDriver;

#[cfg(any(feature = "simulator", feature = "tspi"))]
mod linux;
#[cfg(any(feature = "simulator", feature = "tspi"))]
pub use linux::FileConfigStorage as DefaultConfigStorageDriver;
