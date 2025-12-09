// src/core/data/types.rs
//! 数据类型定义模块
//! 定义数据源系统中使用的核心数据类型

use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::Instant;
use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

use crate::{
    common::{
        GlobalMutex,
        error::{AppError, Result},
    },
    kernel::system::api::DefaultSystemApi,
};

/// 数据源缓存结构
#[derive(Debug, Clone)]
pub struct DataSourceCache {
    /// 字段名称到值的映射
    pub fields: Vec<(String<32>, DynamicValue), 32>,
    /// 上次更新时间戳
    pub last_updated: Instant,
    /// 缓存是否有效
    pub valid: bool,
}

impl Default for DataSourceCache {
    fn default() -> Self {
        Self {
            fields: Vec::new(),
            last_updated: Instant::MIN,
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
    pub fn mark_valid(&mut self, timestamp: Instant) {
        self.last_updated = timestamp;
        self.valid = true;
    }

    /// 标记缓存为无效
    pub fn mark_invalid(&mut self) {
        self.valid = false;
    }
}

pub type DataSourceId = u8;

/// 数据源接口定义
#[async_trait(?Send)]
pub trait DataSource {
    /// 获取数据源名称
    fn name(&self) -> &'static str;

    /// 获取字段值
    fn get_field_value(&self, name: &str) -> Result<DynamicValue>;

    /// 刷新数据源
    async fn refresh(&mut self, system_api: &'static GlobalMutex<DefaultSystemApi>) -> Result<()>;

    /// 获取刷新间隔（秒）
    fn refresh_interval(&self) -> u32;
}

/// 动态值枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DynamicValue {
    /// 布尔值
    Boolean(bool),
    /// 整数
    Integer(i32),
    /// 浮点数
    Float(f32),
    /// 字符串
    String(String<64>),
    // 数组
    // Array(Vec<&'a DynamicValue<'a>, 16>),
}

/// 字段元数据结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMeta {
    /// 字段名称
    pub name: String<32>,
    /// 字段类型
    pub content: DynamicValue,
}
