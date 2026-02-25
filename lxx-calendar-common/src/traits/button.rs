use embassy_executor::task;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonEvent {
    DoubleClick,
    TripleClick,
    ShortPress,
    LongPress,
}

pub const DOUBLE_CLICK_INTERVAL_MS: u32 = 500;
pub const TRIPLE_CLICK_INTERVAL_MS: u32 = 500;
pub const SHORT_PRESS_MAX_MS: u32 = 200;
pub const LONG_PRESS_MIN_MS: u32 = 15000;
pub const DEBOUNCE_MS: u32 = 50;

pub trait ButtonDriver {
    type Error;

    async fn register_press_callback<F>(&mut self, callback: F) -> Result<(), Self::Error>
    where
        F: Fn(ButtonEvent) + Send + 'static;
}

pub struct NoButtonDriver {}

impl ButtonDriver for NoButtonDriver {
    type Error = core::convert::Infallible;

    async fn register_press_callback<F>(&mut self, _callback: F) -> Result<(), Self::Error>
    where
        F: Fn(ButtonEvent) + Send + 'static,
    {
        Ok(())
    }
}
