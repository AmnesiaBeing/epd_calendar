use embassy_executor::Spawner;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::Duration;
use embedded_storage_async::nor_flash::NorFlash;
use serde::{Deserialize, Serialize};

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

/// 睡眠模式
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SleepMode {
    /// Light Sleep: 从暂停点继续
    LightSleep,
    /// Deep Sleep: 从头执行
    DeepSleep,
}

/// 唤醒源
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
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

/// RTC 内存数据（Deep Sleep 后保留）
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct RtcMemoryData {
    /// 唤醒源
    pub wakeup_source: WakeupSource,
    /// 上次更新时间戳
    pub last_update_time: u64,
    /// 配置哈希
    pub config_hash: u32,
    /// 魔数（验证数据有效性）
    pub magic: u32,
}

impl RtcMemoryData {
    pub const MAGIC: u32 = 0x4C585852; // "LXXR"

    pub fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            ..Default::default()
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

/// 平台睡眠管理 Trait
pub trait SleepManager: Send + Sync {
    type Error: core::fmt::Debug + Send;

    /// 进入睡眠
    async fn sleep(
        &mut self,
        mode: SleepMode,
        duration: Duration,
    ) -> Result<WakeupSource, Self::Error>;

    /// 获取上次唤醒源
    fn get_wakeup_source(&self) -> WakeupSource;

    /// 保存 RTC 内存数据
    fn save_rtc_memory(&mut self, data: RtcMemoryData) -> Result<(), Self::Error>;

    /// 读取 RTC 内存数据
    fn load_rtc_memory(&self) -> Result<RtcMemoryData, Self::Error>;
}

pub trait PlatformTrait: Sized {
    fn init_logger() {}

    fn init_heap() {}

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>>;

    fn sys_reset();

    /// 获取唤醒源（从 RTC 内存）
    fn get_wakeup_source() -> WakeupSource {
        WakeupSource::PowerOn
    }

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

/// 虚拟睡眠管理器（用于未实现睡眠管理的平台）
pub struct UnimplementedSleepManager;

impl SleepManager for UnimplementedSleepManager {
    type Error = core::convert::Infallible;

    async fn sleep(
        &mut self,
        _mode: SleepMode,
        _duration: Duration,
    ) -> Result<WakeupSource, Self::Error> {
        // 默认不睡眠，直接返回
        Ok(WakeupSource::PowerOn)
    }

    fn get_wakeup_source(&self) -> WakeupSource {
        WakeupSource::PowerOn
    }

    fn save_rtc_memory(&mut self, _data: RtcMemoryData) -> Result<(), Self::Error> {
        Ok(())
    }

    fn load_rtc_memory(&self) -> Result<RtcMemoryData, Self::Error> {
        Ok(RtcMemoryData::new())
    }
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
