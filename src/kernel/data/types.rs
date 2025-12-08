// src/core/data/types.rs
//! 数据类型定义模块
//! 定义数据源系统中使用的核心数据类型

use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

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

/// 数据源ID枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataSourceId {
    /// 系统数据源
    System = 0x01,
    /// 时间数据源
    Time = 0x02,
    /// 天气数据源
    Weather = 0x04,
    /// 配置数据源
    Config = 0x08,
    /// 名言数据源
    Quote = 0x10,
}

impl DataSourceId {
    /// 获取数据源ID的字符串别名
    pub fn as_str(&self) -> &'static str {
        match self {
            DataSourceId::System => "system",
            DataSourceId::Time => "time",
            DataSourceId::Weather => "weather",
            DataSourceId::Config => "config",
            DataSourceId::Quote => "quote",
        }
    }
}

impl From<DataSourceId> for u8 {
    fn from(id: DataSourceId) -> Self {
        id as u8
    }
}
