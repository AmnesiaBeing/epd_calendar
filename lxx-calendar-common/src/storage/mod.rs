pub mod config_persistence;
pub mod log_storage;

pub use config_persistence::{ConfigPersistence, FlashDevice};
pub use log_storage::{LogEntry, LogLevel, LogStorage, LogStorageStats};