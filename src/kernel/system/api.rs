// src/core/system/api.rs
//! 系统API接口模块
//! 定义系统级别的模块化API接口，包含硬件、网络和配置存储子接口

use crate::common::GlobalMutex;
use crate::common::config::{CONFIG_MAGIC, SystemConfig};
use crate::common::error::{AppError, Result};
use crate::kernel::driver::network::{DefaultNetworkDriver, NetworkDriver};
use crate::kernel::driver::power::{DefaultPowerDriver, PowerDriver};
use crate::kernel::driver::storage::{ConfigStorage, DefaultConfigStorage};

use heapless::String;
use postcard::from_bytes;

/// 硬件API接口
/// 处理硬件相关操作：电池/充电状态、系统时间戳/tick、WiFi连接/状态等
pub trait HardwareApi {
    /// 获取电池电量
    fn get_battery_level(&self) -> u8;

    /// 获取充电状态
    fn is_charging(&self) -> bool;

    /// 检查WiFi连接状态
    fn is_wifi_connected(&self) -> bool;

    /// 连接到WiFi
    async fn connect_wifi(&self, ssid: &str, password: &str) -> Result<()>;

    /// 断开WiFi连接
    async fn disconnect_wifi(&self) -> Result<()>;
}

/// 网络客户端API接口
/// 专做网络请求：HTTP GET/POST，纯数据传输，不涉及硬件控制
pub trait NetworkClientApi {
    /// 发送HTTP GET请求
    async fn http_get(&self, url: &str) -> Result<String<256>>;

    /// 发送HTTP POST请求
    async fn http_post(&self, url: &str, body: &[u8]) -> Result<String<256>>;
}

/// 配置存储API接口
/// 底层配置存储：仅做配置的原始读写（无默认值、无缓存），对接存储驱动
pub trait ConfigApi {
    /// 读取配置数据
    fn read_config(&self) -> Result<Option<heapless::Vec<u8, 1024>>>;

    /// 写入配置数据
    fn write_config(&self, data: &[u8]) -> Result<()>;

    /// 删除配置数据
    fn delete_config(&self) -> Result<()>;
}

/// 系统API接口
/// 聚合接口：提供子接口的统一访问入口，无独立逻辑
pub trait SystemApi {
    /// 获取硬件API实例
    fn get_hardware_api(&self) -> &dyn HardwareApi;

    /// 获取网络客户端API实例
    fn get_network_client_api(&self) -> &dyn NetworkClientApi;

    /// 获取配置存储API实例
    fn get_config_api(&self) -> &dyn ConfigApi;
}

/// 默认系统API实现
pub struct DefaultSystemApi {
    /// 电源驱动实例
    power_driver: GlobalMutex<DefaultPowerDriver>,
    /// 网络驱动实例（全局互斥锁保护）
    network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
    /// 配置存储驱动实例
    config_storage: GlobalMutex<DefaultConfigStorage>,
    /// 当前配置
    current_config: GlobalMutex<SystemConfig>,
    /// 配置是否已修改但未保存
    config_dirty: GlobalMutex<bool>,
}

impl DefaultSystemApi {
    /// 创建新的默认系统API实例
    pub fn new(
        power_driver: DefaultPowerDriver,
        network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
        config_storage: DefaultConfigStorage,
    ) -> Self {
        Self {
            power_driver: GlobalMutex::new(power_driver),
            network_driver,
            config_storage: GlobalMutex::new(config_storage),
            current_config: GlobalMutex::new(SystemConfig::default()),
            config_dirty: GlobalMutex::new(false),
        }
    }

    /// 验证并加载配置数据
    fn validate_and_load_config(&self, data: &[u8]) -> Result<SystemConfig> {
        if data.len() < 8 {
            return Err(AppError::ConfigInvalid);
        }

        // 检查魔法数字
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != CONFIG_MAGIC {
            log::warn!(
                "Invalid config magic: 0x{:08X}, expected 0x{:08X}",
                magic,
                CONFIG_MAGIC
            );
            return Err(AppError::ConfigInvalid);
        }

        // 检查数据大小
        let data_len = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        if data_len + 8 != data.len() {
            log::warn!(
                "Config size mismatch: expected {}, got {}",
                data_len + 8,
                data.len()
            );
            return Err(AppError::ConfigInvalid);
        }

        // 反序列化配置
        let config = from_bytes(&data[8..]).map_err(|e| {
            log::error!("Failed to deserialize config: {:?}", e);
            AppError::ConfigDeserializationError
        })?;

        log::info!(
            "Config loaded successfully, version: {}",
            config.config_version
        );
        Ok(config)
    }

    /// 初始化配置
    pub fn init_config(&self) -> Result<()> {
        match self.config_storage.lock().read_config_block()? {
            Some(data) => {
                // 验证数据有效性并加载配置
                let config = self.validate_and_load_config(&data)?;
                *self.current_config.lock() = config;
                Ok(())
            }
            None => {
                // 没有存储的配置，使用默认值
                log::info!("No stored config found, using defaults");
                Ok(())
            }
        }
    }
}

impl SystemApi for DefaultSystemApi {
    fn get_hardware_api(&self) -> &dyn HardwareApi {
        // 返回硬件API实现
        &self
    }

    fn get_network_client_api(&self) -> &dyn NetworkClientApi {
        // 返回网络客户端API实现
        &self
    }

    fn get_config_api(&self) -> &dyn ConfigApi {
        // 返回配置存储API实现
        &self
    }
}

// 硬件API实现
impl HardwareApi for DefaultSystemApi {
    fn get_battery_level(&self) -> u8 {
        // 获取电池电量
        100
    }

    fn is_charging(&self) -> bool {
        // 获取充电状态
        false
    }

    fn is_wifi_connected(&self) -> bool {
        // 检查WiFi连接状态
        self.network_driver.blocking_lock().is_connected()
    }

    async fn connect_wifi(&self, ssid: &str, password: &str) -> Result<()> {
        // 连接到WiFi
        self.network_driver
            .blocking_lock()
            .connect(ssid, password)
            .await
    }

    async fn disconnect_wifi(&self) -> Result<()> {
        // 断开WiFi连接
        self.network_driver.blocking_lock().disconnect().await
    }
}

// 网络客户端API实现
impl NetworkClientApi for DefaultSystemApi {
    async fn http_get(&self, url: &str) -> Result<String<256>> {
        // 发送HTTP GET请求
        self.network_driver.blocking_lock().http_get(url).await
    }

    async fn http_post(&self, url: &str, body: &[u8]) -> Result<String<256>> {
        // 发送HTTP POST请求
        self.network_driver
            .blocking_lock()
            .http_post(url, body)
            .await
    }
}

// 配置存储API实现
impl ConfigApi for DefaultSystemApi {
    fn read_config(&self) -> Result<Option<heapless::Vec<u8, 1024>>> {
        // 读取配置数据
        match self.config_storage.lock().read_config_block()? {
            Some(data) => {
                let mut result = heapless::Vec::new();
                result
                    .extend_from_slice(&data)
                    .map_err(|_| AppError::ConfigTooLarge)?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    fn write_config(&self, data: &[u8]) -> Result<()> {
        // 写入配置数据
        self.config_storage.lock().write_config_block(data)
    }

    fn delete_config(&self) -> Result<()> {
        // 删除配置数据
        unimplemented!()
    }
}
