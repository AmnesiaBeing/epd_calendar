use crate::types::Melody;

pub trait BuzzerDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn play_melody(&mut self, melody: Melody) -> Result<(), Self::Error>;

    async fn play_tone(&mut self, frequency: u32, duration: embassy_time::Duration) -> Result<(), Self::Error>;

    async fn stop(&mut self) -> Result<(), Self::Error>;

    async fn is_playing(&self) -> Result<bool, Self::Error>;
}
