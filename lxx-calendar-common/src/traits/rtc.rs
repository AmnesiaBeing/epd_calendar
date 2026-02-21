use embassy_time::Duration;

pub trait Rtc: Send + Sync {
    type Error;

    async fn get_time(&self) -> Result<i64, Self::Error>;

    async fn set_time(&mut self, timestamp: i64) -> Result<(), Self::Error>;

    async fn set_wakeup(&mut self, duration: Duration) -> Result<(), Self::Error>;

    async fn sleep_light(&mut self);
}
