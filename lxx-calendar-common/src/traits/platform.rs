use embassy_executor::Spawner;
use embassy_sync::channel::{Channel, Receiver, Sender};

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

pub trait PlatformTrait: Sized {
    fn init_logger() {}

    fn init_heap() {}

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>>;

    fn sys_reset();

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
}
