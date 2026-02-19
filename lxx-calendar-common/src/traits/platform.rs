use embassy_sync::channel::{Channel, Receiver, Sender};

use super::{BuzzerDriver, Rtc, Watchdog};

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

    /// 看门狗设备，必须实现 Watchdog trait
    type WatchdogDevice: Watchdog;

    /// EPD 设备
    type EpdDevice;

    /// 音频/蜂鸣器设备
    type AudioDevice: BuzzerDriver;

    /// RTC 设备
    type RtcDevice: Rtc;
}

pub struct PlatformContext<C: PlatformTrait + Sized> {
    /// 看门狗设备
    pub sys_watch_dog: C::WatchdogDevice,
    /// EPD 设备
    pub epd: C::EpdDevice,
    /// 音频设备
    pub audio: C::AudioDevice,
    /// RTC 设备
    pub rtc: C::RtcDevice,
}
