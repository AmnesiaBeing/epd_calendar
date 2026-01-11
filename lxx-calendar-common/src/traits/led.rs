pub trait LEDDriver {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn set_on(&mut self) -> Result<(), Self::Error>;

    async fn set_off(&mut self) -> Result<(), Self::Error>;

    async fn toggle(&mut self) -> Result<(), Self::Error>;

    async fn is_on(&self) -> Result<bool, Self::Error>;
}
