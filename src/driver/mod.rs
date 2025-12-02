// src/driver/mod.rs
pub mod display;
pub mod network;
pub mod ntp_source;
pub mod power;
pub mod sensor;
pub mod storage;
pub mod time_source;

#[cfg(feature = "embedded_esp")]
pub mod rng;
