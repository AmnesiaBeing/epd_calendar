pub trait BuzzerDriver {
    type Error;

    fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), Self::Error>;
}
