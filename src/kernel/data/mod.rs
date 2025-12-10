// src/core/data/mod.rs
//! 数据模块
//! 提供数据源、数据类型和数据注册表的定义

pub mod sources;
pub mod types;

pub use types::{DataSource, DataSourceCache};
pub use types::{DataSourceId, DynamicValue};

pub mod scheduler;
pub use scheduler::{DataSourceRegistry, generic_scheduler_task};
