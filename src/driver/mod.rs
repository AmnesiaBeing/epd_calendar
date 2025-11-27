// src/driver/mod.rs
pub mod display;
pub mod network;
pub mod power;
pub mod sensor;
pub mod storage;
pub mod time_source;

// 重新导出常用驱动
pub use display::DisplayDriver;
// pub use network::NetworkDriver;
// pub use power::PowerMonitor;
// pub use sensor::SensorDriver;
pub use storage::StorageDriver;
pub use time_source::TimeSource;
