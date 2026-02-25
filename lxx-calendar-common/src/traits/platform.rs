use embassy_executor::Spawner;
use embassy_sync::channel::{Channel, Receiver, Sender};

use crate::{SystemEvent, SystemResult};

use super::{
    Battery, ButtonDriver, BuzzerDriver, LEDDriver, NetworkStack, Rtc, Watchdog, WifiController,
};

const CAP: usize = 10;

pub type LxxSystemEventChannel =
    Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, SystemEvent, CAP>;

pub type LxxChannelSender<'a, T> =
    Sender<'a, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, T, CAP>;

pub type LxxChannelReceiver<'a, T> =
    Receiver<'a, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, T, CAP>;

pub trait PlatformTrait: Sized {
    fn init_logger() {}

    fn init_heap() {}

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>>;

    fn sys_reset();

    /// 看门狗设备
    type WatchdogDevice: Watchdog;

    /// 按钮设备
    type ButtonDevice: ButtonDriver;

    /// EPD 设备
    type EpdDevice;

    /// 音频/蜂鸣器设备
    type AudioDevice: BuzzerDriver;

    /// RTC 设备
    type RtcDevice: Rtc;

    /// WiFi 控制器（物理层）
    type WifiDevice: WifiController;

    /// 网络协议栈
    type NetworkStack: NetworkStack;

    /// LED 指示灯设备
    type LEDDevice: LEDDriver;

    /// 电池监控设备
    type BatteryDevice: Battery;
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
    /// WiFi 设备
    pub wifi: C::WifiDevice,
    /// 网络协议栈
    pub network: C::NetworkStack,
    /// LED 指示灯设备
    pub led: C::LEDDevice,
    /// 电池监控设备
    pub battery: C::BatteryDevice,
    /// 按钮设备
    pub button: C::ButtonDevice,
}