// src/core/data/types.rs
//! 数据类型定义模块
//! 定义数据源系统中使用的核心数据类型

use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

/// 字段类型枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldType {
    /// 布尔值类型
    Boolean,
    /// 整数类型
    Integer,
    /// 浮点数类型
    Float,
    /// 字符串类型
    String,
    /// 时间类型
    Time,
    /// 日期类型
    Date,
    /// 农历日期类型
    LunarDate,
    /// 天气数据类型
    Weather,
    /// 配置数据类型
    Config,
}

/// 动态值枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DynamicValue {
    /// 布尔值
    Boolean(bool),
    /// 整数
    Integer(i64),
    /// 浮点数
    Float(f64),
    /// 字符串
    String(String<64>),
    /// 时间数据
    TimeData(u8, u8, Option<bool>), // hour, minute, is_pm
    /// 日期数据
    DateData(u16, u8, u8), // year, month, day
    /// 空值
    None,
}

/// 字段元数据结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMeta {
    /// 字段名称
    pub name: String<32>,
    /// 字段类型
    pub field_type: FieldType,
    /// 字段格式
    pub format: String<16>,
    /// 是否可为空
    pub nullable: bool,
    /// 字段描述
    pub description: String<64>,
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

impl From<u8> for Option<DataSourceId> {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Some(DataSourceId::System),
            0x02 => Some(DataSourceId::Time),
            0x04 => Some(DataSourceId::Weather),
            0x08 => Some(DataSourceId::Config),
            0x10 => Some(DataSourceId::Quote),
            _ => None,
        }
    }
}