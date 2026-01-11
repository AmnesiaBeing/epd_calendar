pub trait BatteryMonitor {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn read_voltage(&self) -> Result<u16, Self::Error>;

    async fn read_percentage(&self) -> Result<u8, Self::Error>;

    async fn is_low_battery(&self) -> Result<bool, Self::Error>;
}
