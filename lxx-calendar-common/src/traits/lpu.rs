use embassy_sync::channel::Sender;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

pub trait LPUCore {
    type Error;

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn start_monitoring(&mut self) -> Result<(), Self::Error>;

    async fn stop_monitoring(&mut self) -> Result<(), Self::Error>;

    async fn is_monitoring(&self) -> Result<bool, Self::Error>;

    async fn get_event_sender(&self) -> Result<Option<Sender<'static, CriticalSectionRawMutex, crate::SystemEvent, 32>>, Self::Error>;
}
