// src/core/data/mod.rs
//! 数据模块
//! 提供数据源、数据类型和数据注册表的定义

pub mod registry;
pub mod source;
pub mod sources;
pub mod types;

pub use registry::DataSourceRegistry;
pub use source::{DataSource, DataSourceCache};
pub use types::{DataSourceId, DynamicValue, FieldMeta, FieldType};

pub mod scheduler;
pub use scheduler::{DataSourceEvent, DataSourceScheduler, generic_scheduler_task};
