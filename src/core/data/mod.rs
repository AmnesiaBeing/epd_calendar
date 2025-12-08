// src/core/data/mod.rs
//! 数据模块
//! 提供数据源、数据类型和数据注册表的定义

pub mod registry;
pub mod source;
pub mod types;
pub mod sources;

pub use registry::DataSourceRegistry;
pub use source::{DataSource, DataSourceCache};
pub use types::{DataSourceId, DynamicValue, FieldMeta, FieldType};