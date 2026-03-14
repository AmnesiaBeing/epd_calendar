#![no_std]
#![allow(async_fn_in_trait)]

extern crate alloc;

pub mod compiled_config;
pub mod events;
pub mod flash_layout;
pub mod http;
pub mod sntp;
pub mod storage;
pub mod traits;
pub mod types;
pub mod weather;

cfg_if::cfg_if! {
    if #[cfg(feature = "defmt")] {
        pub use defmt::{trace, debug, info, warn, error};
    }
    else if #[cfg(feature = "log")] {
        pub use log::{trace, debug, info, warn, error};
    }
    else {
        #[doc(hidden)]
        pub mod __private {
            #[macro_export]
            macro_rules! trace_impl { ($($arg:tt)*) => {{}}; }
            #[macro_export]
            macro_rules! debug_impl { ($($arg:tt)*) => {{}}; }
            #[macro_export]
            macro_rules! info_impl { ($($arg:tt)*) => {{}}; }
            #[macro_export]
            macro_rules! warn_impl { ($($arg:tt)*) => {{}}; }
            #[macro_export]
            macro_rules! error_impl { ($($arg:tt)*) => {{}}; }
        }
        pub use __private::{trace_impl as trace, debug_impl as debug, info_impl as info, warn_impl as warn, error_impl as error};
    }
}

pub use events::*;
pub use traits::*;
pub use types::*;
