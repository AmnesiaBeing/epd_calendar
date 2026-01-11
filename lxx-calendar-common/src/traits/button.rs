use embassy_time::Duration;

pub enum ButtonEvent {
    ShortPress,
    LongPress,
}

pub trait ButtonDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn wait_for_press(&mut self, timeout: Duration) -> Result<ButtonEvent, Self::Error>;
}

pub trait ButtonEventTrait {
    fn is_short_press(&self) -> bool;

    fn is_long_press(&self) -> bool;

    fn get_duration(&self) -> Duration;
}

impl ButtonEventTrait for ButtonEvent {
    fn is_short_press(&self) -> bool {
        matches!(self, ButtonEvent::ShortPress)
    }

    fn is_long_press(&self) -> bool {
        matches!(self, ButtonEvent::LongPress)
    }

    fn get_duration(&self) -> Duration {
        match self {
            ButtonEvent::ShortPress => Duration::from_millis(100),
            ButtonEvent::LongPress => Duration::from_secs(15),
        }
    }
}
