// src/core/data/source.rs
//! 数据源定义模块
//! 定义数据源接口和缓存结构

use crate::common::error::{AppError, Result};
use crate::kernel::data::types::{DataSourceId, DynamicValue, FieldMeta};
use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

/// 数据源缓存结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceCache {
    /// 字段名称到值的映射
    pub fields: Vec<(String<32>, DynamicValue), 32>,
    /// 上次更新时间戳
    pub last_updated: u32,
    /// 缓存是否有效
    pub valid: bool,
}

impl Default for DataSourceCache {
    fn default() -> Self {
        Self {
            fields: Vec::new(),
            last_updated: 0,
            valid: false,
        }
    }
}

impl DataSourceCache {
    /// 获取字段值
    pub fn get_field(&self, name: &str) -> Option<&DynamicValue> {
        self.fields
            .iter()
            .find(|(field_name, _)| field_name.as_str() == name)
            .map(|(_, value)| value)
    }

    /// 设置字段值
    pub fn set_field(&mut self, name: String<32>, value: DynamicValue) -> Result<()> {
        // 查找字段是否已存在
        if let Some(index) = self
            .fields
            .iter()
            .position(|(field_name, _)| field_name.as_str() == name.as_str())
        {
            // 更新现有字段
            self.fields[index] = (name, value);
        } else {
            // 添加新字段
            self.fields
                .push((name, value))
                .map_err(|_| AppError::DataCapacityExceeded)?;
        }
        Ok(())
    }

    /// 清除所有字段
    pub fn clear(&mut self) {
        self.fields.clear();
        self.valid = false;
    }

    /// 标记缓存为有效
    pub fn mark_valid(&mut self, timestamp: u32) {
        self.last_updated = timestamp;
        self.valid = true;
    }

    /// 标记缓存为无效
    pub fn mark_invalid(&mut self) {
        self.valid = false;
    }
}

/// 数据源接口定义
pub trait DataSource {
    /// 获取数据源ID
    fn id(&self) -> DataSourceId;

    /// 获取数据源名称
    fn name(&self) -> &'static str;

    /// 获取所有字段元数据
    fn fields(&self) -> &[FieldMeta];

    /// 获取字段值
    async fn get_field_value(&self, name: &str) -> Result<DynamicValue>;

    /// 刷新数据源
    async fn refresh(
        &mut self,
        system_api: &dyn crate::kernel::system::api::SystemApi,
    ) -> Result<()>;

    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> u32;

    /// 检查数据是否有效
    fn is_data_valid(&self) -> bool;

    /// 获取缓存
    fn get_cache(&self) -> &DataSourceCache;

    /// 获取可变缓存（仅用于内部实现）
    fn get_cache_mut(&mut self) -> &mut DataSourceCache;
}
