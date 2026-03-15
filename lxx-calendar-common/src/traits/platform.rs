use embassy_executor::Spawner;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::Duration;
use embedded_storage_async::nor_flash::NorFlash;

use crate::{SystemEvent, SystemResult};

use super::{
    BLEDriver, Battery, ButtonDriver, BuzzerDriver, LEDDriver, NetworkStack, OTADriver, Rtc,
    Watchdog, WifiController,
};

const CAP: usize = 10;

pub type LxxSystemEventChannel =
    Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, SystemEvent, CAP>;

pub type LxxChannelSender<'a, T> =
    Sender<'a, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, T, CAP>;

pub type LxxChannelReceiver<'a, T> =
    Receiver<'a, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, T, CAP>;

/// 唤醒源
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WakeupSource {
    /// 首次上电
    PowerOn,
    /// RTC 定时器唤醒
    RtcTimer,
    /// 按键唤醒
    Button,
    /// 看门狗唤醒
    Watchdog,
}

impl Default for WakeupSource {
    fn default() -> Self {
        WakeupSource::PowerOn
    }
}

pub trait PlatformTrait: Sized {
    fn init_logger() {}

    fn init_heap() {}

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>>;

    fn sys_reset();

    /// 获取唤醒源
    fn get_wakeup_source() -> WakeupSource {
        WakeupSource::PowerOn
    }

    /// 进入 Deep Sleep，指定唤醒时间
    async fn deep_sleep(duration: Duration) -> WakeupSource;

    type WatchdogDevice: Watchdog;

    type ButtonDevice: ButtonDriver;

    type EpdDevice;

    type AudioDevice: BuzzerDriver;

    type RtcDevice: Rtc;

    type WifiDevice: WifiController;

    type NetworkStack: NetworkStack;

    type LEDDevice: LEDDriver;

    type BatteryDevice: Battery;

    type BLEDevice: BLEDriver;

    type OTADevice: OTADriver;

    type FlashDevice: NorFlash;
}

pub struct PlatformContext<C: PlatformTrait + Sized> {
    pub sys_watch_dog: C::WatchdogDevice,
    pub epd: C::EpdDevice,
    pub audio: C::AudioDevice,
    pub rtc: C::RtcDevice,
    pub wifi: C::WifiDevice,
    pub network: C::NetworkStack,
    pub led: C::LEDDevice,
    pub battery: C::BatteryDevice,
    pub button: C::ButtonDevice,
    pub ble: C::BLEDevice,
    pub ota: C::OTADevice,
    pub flash: C::FlashDevice,
}

impl<C: PlatformTrait> PlatformContext<C> {
    pub fn led(&self) -> &C::LEDDevice {
        &self.led
    }

    pub fn led_mut(&mut self) -> &mut C::LEDDevice {
        &mut self.led
    }
}
