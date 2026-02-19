#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
#![no_std]
#![no_main]

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use esp_hal::ledc::{self, Ledc, LowSpeed, channel};
use esp_hal::timer::timg::{MwdtStage, TimerGroup, Wdt};
use esp_hal::peripherals::GPIO7;
use esp_hal::rtc_cntl::Rtc;
pub use esp_rtos::main as platform_main;
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use static_cell::StaticCell;

const RTC_STORAGE_MAGIC: u32 = 0x52544350; // "RTCP"
const RTC_STORAGE_KEY: u8 = 0x00;

#[repr(C)]
struct RtcStorage {
    magic: u32,
    timestamp: i64,
}

esp_bootloader_esp_idf::esp_app_desc!();

use panic_rtt_target as _;

pub struct Esp32Watchdog {
    inner: Wdt<esp_hal::peripherals::TIMG0<'static>>,
}

impl Esp32Watchdog {
    pub fn new(wdt: Wdt<esp_hal::peripherals::TIMG0<'static>>) -> Self {
        Self { inner: wdt }
    }
}

impl Watchdog for Esp32Watchdog {
    type Error = core::convert::Infallible;

    fn feed(&mut self) -> Result<(), Self::Error> {
        self.inner.feed();
        Ok(())
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        self.inner.enable();
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Self::Error> {
        self.inner.disable();
        Ok(())
    }

    fn get_timeout(&self) -> Result<u32, Self::Error> {
        Ok(0)
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<(), Self::Error> {
        let timeout_us = timeout_ms as u64 * 1000;
        self.inner.set_timeout(
            MwdtStage::Stage0,
            esp_hal::time::Duration::from_micros(timeout_us),
        );
        Ok(())
    }
}

pub struct Esp32Buzzer {
    ledc: Ledc<'static>,
    pin: GPIO7<'static>,
}

impl Esp32Buzzer {
    pub fn new(ledc: Ledc<'static>, pin: GPIO7<'static>) -> Self {
        Self { ledc, pin }
    }
}

impl BuzzerDriver for Esp32Buzzer {
    type Error = core::convert::Infallible;

    fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), Self::Error> {
        use esp_hal::ledc::timer::TimerIFace;
        use esp_hal::ledc::channel::ChannelIFace;
        use esp_hal::time::Rate;
        use embassy_time::block_for;

        self.ledc.set_global_slow_clock(ledc::LSGlobalClkSource::APBClk);
        let mut timer = self.ledc.timer::<LowSpeed>(ledc::timer::Number::Timer0);

        let timer_config = ledc::timer::config::Config {
            duty: ledc::timer::config::Duty::Duty10Bit,
            clock_source: ledc::timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(frequency),
        };
        timer.configure(timer_config).ok();

        let mut ch = self.ledc.channel(channel::Number::Channel0, self.pin.reborrow());
        let ch_config = channel::config::Config {
            timer: &timer,
            duty_pct: 50,
            drive_mode: esp_hal::gpio::DriveMode::PushPull,
        };
        ch.configure(ch_config).ok();

        block_for(embassy_time::Duration::from_millis(duration_ms as u64));

        ch.configure(channel::config::Config {
            timer: &timer,
            duty_pct: 0,
            drive_mode: esp_hal::gpio::DriveMode::PushPull,
        }).ok();

        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn is_playing(&self) -> bool {
        false
    }
}

pub struct Esp32Rtc {
    base_timestamp: i64,
    boot_instant: embassy_time::Instant,
}

impl Esp32Rtc {
    pub fn new() -> Self {
        Self {
            base_timestamp: 1704067200,
            boot_instant: embassy_time::Instant::now(),
        }
    }

    fn load_timestamp() -> i64 {
        let mut storage = RtcStorage {
            magic: 0,
            timestamp: 0,
        };
        if let Ok(len) = esp_storage::read(rtc::FlashStorageAddress::BootloadDERaseSize0.get(), &mut storage) {
            if len >= core::mem::size_of::<RtcStorage>() && storage.magic == RTC_STORAGE_MAGIC {
                return storage.timestamp;
            }
        }
        0
    }

    fn save_timestamp(timestamp: i64) {
        let storage = RtcStorage {
            magic: RTC_STORAGE_MAGIC,
            timestamp,
        };
        let mut buf = [0u8; 64];
        let src = &storage as *const RtcStorage as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), core::mem::size_of::<RtcStorage>());
        }
        let _ = esp_storage::write(rtc::FlashStorageAddress::BootloadDERaseSize0.get(), &buf);
    }
}

impl Default for Esp32Rtc {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rtc for Esp32Rtc {
    type Error = core::convert::Infallible;

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        let stored = Self::load_timestamp();
        if stored > 0 {
            self.base_timestamp = stored;
        } else {
            self.base_timestamp = 1704067200;
        }
        self.boot_instant = embassy_time::Instant::now();
        info!("ESP32 RTC initialized with base timestamp: {}", self.base_timestamp);
        Ok(())
    }

    async fn get_time(&self) -> Result<i64, Self::Error> {
        let elapsed = self.boot_instant.elapsed().as_secs() as i64;
        Ok(self.base_timestamp + elapsed)
    }

    async fn set_time(&mut self, timestamp: i64) -> Result<(), Self::Error> {
        self.base_timestamp = timestamp;
        Self::save_timestamp(timestamp);
        self.boot_instant = embassy_time::Instant::now();
        info!("ESP32 RTC time set to: {}", timestamp);
        Ok(())
    }
}

pub struct Esp32Wifi {
    connected: bool,
}

impl Esp32Wifi {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

impl Default for Esp32Wifi {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WifiController for Esp32Wifi {
    type Error = core::convert::Infallible;

    async fn connect_sta(&mut self, ssid: &str, password: &str) -> Result<(), Self::Error> {
        info!("ESP32 WiFi connecting to SSID: {}", ssid);
        // TODO: 使用 esp-radio 实现真正的 WiFi 连接
        // 需要使用 esp_radio::wifi 模块来连接 WiFi
        // 示例代码：
        // let config = ClientConfiguration { ... };
        // esp_radio::wifi::sta_connect(config).await?;
        self.connected = true;
        info!("ESP32 WiFi connected (stub)");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        info!("ESP32 WiFi disconnecting");
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn get_rssi(&self) -> Result<i32, Self::Error> {
        Ok(-50)
    }
}

pub struct Esp32Network;

impl Esp32Network {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Esp32Network {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NetworkStack for Esp32Network {
    type Error = core::convert::Infallible;

    async fn dns_query(&self, _host: &str) -> Result<Vec<core::net::IpAddr>, Self::Error> {
        // TODO: 使用 embassy-net 的 DNS 功能
        // 需要获取 embassy_net::Stack 的引用
        info!("DNS query (stub)");
        Ok(vec![])
    }

    fn is_link_up(&self) -> bool {
        true
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        info!("Waiting for network config (stub)");
        Ok(())
    }

    fn is_config_up(&self) -> bool {
        true
    }
}

pub struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = Esp32Watchdog;

    type EpdDevice = epd_yrd0750ryf665f60::yrd0750ryf665f60::Epd7in5<
        embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
            'static,
            CriticalSectionRawMutex,
            esp_hal::spi::master::Spi<'static, esp_hal::Async>,
            esp_hal::gpio::Output<'static>,
        >,
        esp_hal::gpio::Input<'static>,
        esp_hal::gpio::Output<'static>,
        esp_hal::gpio::Output<'static>,
        embassy_time::Delay,
    >;

    type AudioDevice = Esp32Buzzer;

    type RtcDevice = Esp32Rtc;

    type WifiDevice = Esp32Wifi;

    type NetworkStack = Esp32Network;

    async fn init(_spawner: embassy_executor::Spawner) -> PlatformContext<Self> {
        static SPI_BUS_MUTEX: StaticCell<
            embassy_sync::mutex::Mutex<
                CriticalSectionRawMutex,
                esp_hal::spi::master::Spi<'static, esp_hal::Async>,
            >,
        > = StaticCell::new();
        static EPD_DEVICE: StaticCell<
            embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
                CriticalSectionRawMutex,
                esp_hal::spi::master::Spi<'static, esp_hal::Async>,
                esp_hal::gpio::Output<'static>,
            >,
        > = StaticCell::new();

        let peripherals = esp_hal::init(
            esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max()),
        );
        esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 32768);

        let timg0 = TimerGroup::new(peripherals.TIMG0);
        let wdt = timg0.wdt;
        let sys_watch_dog = Esp32Watchdog::new(wdt);

        let sck = peripherals.GPIO22;
        let sda = peripherals.GPIO23;
        let cs: esp_hal::gpio::Output<'_> = esp_hal::gpio::Output::new(
            peripherals.GPIO21,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );

        let busy =
            esp_hal::gpio::Input::new(peripherals.GPIO18, esp_hal::gpio::InputConfig::default());
        let dc = esp_hal::gpio::Output::new(
            peripherals.GPIO20,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );
        let rst = esp_hal::gpio::Output::new(
            peripherals.GPIO19,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );

        let spi2 = peripherals.SPI2;

        let spi_bus = esp_hal::spi::master::Spi::new(
            spi2,
            esp_hal::spi::master::Config::default()
                .with_frequency(esp_hal::time::Rate::from_mhz(10))
                .with_mode(esp_hal::spi::Mode::_0),
        )
        .unwrap()
        .with_sck(sck)
        .with_sio0(sda)
        .into_async();

        let spi_bus_mutex = embassy_sync::mutex::Mutex::new(spi_bus);
        let spi_bus_mutex_static: &'static _ = SPI_BUS_MUTEX.init(spi_bus_mutex);

        let epd_device =
            embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(spi_bus_mutex_static, cs);
        let epd_device_static: &'static mut _ = EPD_DEVICE.init(epd_device);

        let mut delay = embassy_time::Delay;

        let epd = Epd7in5::new(epd_device_static, busy, dc, rst, &mut delay)
            .await
            .unwrap();

        let ledc = esp_hal::ledc::Ledc::new(peripherals.LEDC);
        let audio = Esp32Buzzer::new(ledc, peripherals.GPIO7);

        let mut rtc = Esp32Rtc::new();
        rtc.initialize().await.ok();

        let wifi = Esp32Wifi::new();
        let network = Esp32Network::new();

        PlatformContext {
            sys_watch_dog,
            epd,
            audio,
            rtc,
            wifi,
            network,
        }
    }

    fn sys_reset() {
        todo!()
    }

    fn sys_stop() {
        todo!()
    }
}

#[platform_main]
async fn main(spawner: embassy_executor::Spawner) {
    let platform_ctx = Platform::init(spawner).await;
    if let Err(e) = main_task::<Platform>(spawner, platform_ctx).await {
        error!("Main task error: {:?}", e);
    }
}
