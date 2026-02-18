use embassy_sync::channel::{Channel, Receiver, Sender};

const CAP: usize = 10;

pub type LxxSystemEventChannel = Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    crate::events::SystemEvent,
    CAP,
>;

pub type LxxChannelSender<'a, T> =
    Sender<'a, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, T, CAP>;

pub type LxxChannelReceiver<'a, T> =
    Receiver<'a, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, T, CAP>;

pub trait PlatformTrait: Sized {
    fn init_logger() {}

    fn init_heap() {}

    fn init(
        spawner: embassy_executor::Spawner,
    ) -> impl core::future::Future<Output = PlatformContext<Self>>;

    fn sys_reset();

    fn sys_stop();

    type WatchdogDevice;

    type EpdDevice;
}

pub struct PlatformContext<C: PlatformTrait + Sized> {
    pub sys_watch_dog: C::WatchdogDevice,
    pub epd: C::EpdDevice,
}
