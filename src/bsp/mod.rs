#[cfg(feature = "simulator")]
pub mod simulator;

#[cfg(feature = "simulator")]
pub use simulator::Board;

#[cfg(feature = "embedded_linux")]
pub mod embedded_linux;

#[cfg(feature = "embedded_linux")]
pub use embedded_linux::Board;
