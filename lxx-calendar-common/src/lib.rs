#![no_std]
#![allow(async_fn_in_trait)]

extern crate alloc;

pub mod events;
pub mod http;
pub mod sntp;
pub mod traits;
pub mod types;
pub mod weather;

cfg_if::cfg_if! {
    if #[cfg(feature = "log")] {
        pub use log::{trace, debug, info, warn, error};
    }
    else if #[cfg(feature = "defmt")] {
        pub use defmt::{trace, debug, info, warn, error};
    }
    else {
        #[macro_export]
        macro_rules! trace {
            ($($arg:tt)*) => {{}};
        }
        #[macro_export]
        macro_rules! debug {
            ($($arg:tt)*) => {{}};
        }
        #[macro_export]
        macro_rules! info {
            ($($arg:tt)*) => {{}};
        }
        #[macro_export]
        macro_rules! warn {
            ($($arg:tt)*) => {{}};
        }
        #[macro_export]
        macro_rules! error {
            ($($arg:tt)*) => {{}};
        }
    }
}

pub use events::*;
pub use traits::*;
pub use types::*;
