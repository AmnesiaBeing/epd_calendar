// src/kernel/data/registry.rs
//! 数据源注册表模块
//! 管理所有数据源实例的注册表

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::data::source::DataSource;
use crate::kernel::data::types::DataSourceId;
use heapless::Vec;

/// 数据源注册表
pub struct DataSourceRegistry {
    /// 数据源列表
    data_sources: Vec<&'static GlobalMutex<dyn DataSource + Send>, 8>,
}

impl Default for DataSourceRegistry {
    fn default() -> Self {
        Self {
            data_sources: Vec::new(),
        }
    }
}

impl DataSourceRegistry {
    /// 创建新的数据源注册表
    pub fn new() -> Self {
        Default::default()
    }

    /// 注册数据源
    pub fn register_data_source(
        &mut self,
        data_source: &'static GlobalMutex<dyn DataSource + Send>,
    ) -> Result<()> {
        // 检查是否已注册
        let id = data_source.lock().id();
        if self.data_sources.iter().any(|ds| ds.lock().id() == id) {
            return Err(AppError::DataSourceAlreadyRegistered);
        }

        // 注册数据源
        self.data_sources
            .push(data_source)
            .map_err(|_| AppError::DataCapacityExceeded)?;

        Ok(())
    }

    /// 获取数据源
    pub fn get_data_source(
        &self,
        id: DataSourceId,
    ) -> Option<&'static GlobalMutex<dyn DataSource + Send>> {
        self.data_sources
            .iter()
            .find(|ds| ds.lock().id() == id)
            .copied()
    }

    /// 获取所有数据源
    pub fn get_all_data_sources(&self) -> &[&'static GlobalMutex<dyn DataSource + Send>] {
        &self.data_sources
    }

    /// 刷新所有数据源
    pub async fn refresh_all(
        &self,
        system_api: &dyn crate::kernel::system::api::SystemApi,
    ) -> Result<()> {
        for data_source in &self.data_sources {
            data_source.lock().refresh(system_api).await?;
        }
        Ok(())
    }

    /// 按ID刷新数据源
    pub async fn refresh_by_id(
        &self,
        id: DataSourceId,
        system_api: &dyn crate::kernel::system::api::SystemApi,
    ) -> Result<()> {
        if let Some(data_source) = self.get_data_source(id) {
            data_source.lock().refresh(system_api).await?;
            Ok(())
        } else {
            Err(AppError::DataSourceNotFound)
        }
    }
}

/// 全局数据源注册表实例
pub static DATA_SOURCE_REGISTRY: GlobalMutex<DataSourceRegistry> =
    GlobalMutex::new(DataSourceRegistry::new());
