use embassy_time::Duration;

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

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn wait_for_press(&mut self, timeout: Duration) -> Result<ButtonEvent, Self::Error>;
}

pub trait ButtonEventTrait {
    fn is_short_press(&self) -> bool;
    fn is_long_press(&self) -> bool;
    fn is_triple_click(&self) -> bool;
    fn get_duration(&self) -> Duration;
}

impl ButtonEventTrait for ButtonEvent {
    fn is_short_press(&self) -> bool {
        matches!(self, ButtonEvent::ShortPress)
    }

    fn is_long_press(&self) -> bool {
        matches!(self, ButtonEvent::LongPress)
    }

    fn is_triple_click(&self) -> bool {
        matches!(self, ButtonEvent::TripleClick)
    }

    fn get_duration(&self) -> Duration {
        match self {
            ButtonEvent::ShortPress => Duration::from_millis(100),
            ButtonEvent::LongPress => Duration::from_secs(15),
            _ => Duration::from_millis(0),
        }
    }
}
