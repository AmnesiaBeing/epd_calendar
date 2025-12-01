// src/tasks/mod.rs

pub mod quote_task;
pub mod status_task;
pub mod time_task;
pub mod weather_task;

pub use quote_task::quote_task;
pub use status_task::status_task;
pub use time_task::time_task;
pub use weather_task::weather_task;
