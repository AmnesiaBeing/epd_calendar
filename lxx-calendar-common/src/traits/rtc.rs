#![allow(async_fn_in_trait)]

pub trait Rtc: Send + Sync {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn get_time(&self) -> Result<i64, Self::Error>;

    async fn set_time(&mut self, timestamp: i64) -> Result<(), Self::Error>;
}
