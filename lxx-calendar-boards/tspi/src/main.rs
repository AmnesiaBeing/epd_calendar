use embassy_executor::Spawner;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use linux_embedded_hal::{SpidevDevice, SysfsPin};
use lxx_calendar_common::platform::PlatformTrait;
use lxx_calendar_common::traits::platform::{RtcMemoryData, WakeupSource};
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use simulator::{
    HttpServer, SimulatedBLE, SimulatedFlash, SimulatedRtc, SimulatedWdt, SimulatorButton,
    SimulatorControl,
};
use static_cell::StaticCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tokio::sync::Mutex as TokioMutex;

pub mod drivers;
pub mod sleep;

use crate::drivers::{LinuxBuzzer, LinuxWifi, TspiButton, TspiLED, TunTapNetwork};
use crate::sleep::TspiSleepManager;

static SIMULATOR_CONTROL: StaticCell<Option<Arc<Mutex<SimulatorControl>>>> = StaticCell::new();

fn init_gpio(
    pin: u64,
    direction: linux_embedded_hal::sysfs_gpio::Direction,
) -> Result<SysfsPin, linux_embedded_hal::sysfs_gpio::Error> {
    let gpio = SysfsPin::new(pin);
    gpio.export()?;

    while !gpio.is_exported() {}

    gpio.set_direction(direction)?;

    if direction == linux_embedded_hal::sysfs_gpio::Direction::Out {
        gpio.set_value(1)?;
    }

    Ok(gpio)
}

struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = SimulatedWdt;

    type EpdDevice = SpidevDevice;

    type AudioDevice = LinuxBuzzer;

    type LEDDevice = TspiLED;

    type RtcDevice = SimulatedRtc;

    type WifiDevice = LinuxWifi;

    type NetworkStack = TunTapNetwork;

    type BatteryDevice = NoBattery;

    type ButtonDevice = TspiButton;

    type BLEDevice = NoBLE;

    type OTADevice = NoOTA;

    type FlashDevice = SimulatedFlash;

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>> {
        let epd_busy = init_gpio(101, linux_embedded_hal::sysfs_gpio::Direction::In).unwrap();
        let epd_dc = init_gpio(102, linux_embedded_hal::sysfs_gpio::Direction::Out).unwrap();
        let epd_rst = init_gpio(97, linux_embedded_hal::sysfs_gpio::Direction::Out).unwrap();

        let mut spi = SpidevDevice::open("/dev/spidev3.0").unwrap();

        let mut delay = linux_embedded_hal::Delay;
        let _epd = Epd7in5::new(&mut spi, epd_busy, epd_dc, epd_rst, &mut delay)
            .await
            .unwrap();

        let wdt = SimulatedWdt::new(5000);
        simulator::start_watchdog(&spawner, 5000);

        let audio = LinuxBuzzer;

        let mut rtc = SimulatedRtc::new();
        rtc.initialize().await.ok();

        let wifi = LinuxWifi::new();
        let network = TunTapNetwork::new(spawner)?;
        let led = TspiLED;
        let battery = NoBattery::new(3700, false, false);
        let button = TspiButton::new();

        let control = SIMULATOR_CONTROL.init(None);
        let ble = SimulatedBLE::new();
        let http_button = SimulatorButton::new();

        let flash = SimulatedFlash::new(PathBuf::from("/tmp/tspi_flash.bin"));
        info!("Flash initialized");

        if let Some(ref ctrl) = *control {
            let port = std::env::var("SIMULATOR_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080);

            let ctrl_clone = Arc::clone(ctrl);
            let ble_arc = Arc::new(Mutex::new(ble));
            let ble_clone = Arc::clone(&ble_arc);
            let button_arc = Arc::new(Mutex::new(http_button));
            let button_clone = Arc::clone(&button_arc);
            thread::spawn(move || {
                HttpServer::new(ctrl_clone, ble_clone, button_clone, port).run();
            });
        }

        Ok(PlatformContext {
            sys_watch_dog: wdt,
            epd: spi,
            audio,
            rtc,
            wifi,
            network,
            led,
            battery,
            button,
            ble: NoBLE::new(),
            ota: NoOTA::new(),
            flash,
        })
    }

    fn sys_reset() {
        info!("TSPI platform reset");
    }

    fn init_logger() {
        env_logger::init();
    }

    fn init_heap() {}

    fn get_wakeup_source() -> WakeupSource {
        // T-SPi 默认返回 PowerOn
        // 实际唤醒源从 RTC 内存读取
        WakeupSource::PowerOn
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    Platform::init_heap();
    Platform::init_logger();

    let rtc = SimulatedRtc::new();
    let watchdog = SimulatedWdt::new(5000);

    let simulator_control = Arc::new(Mutex::new(SimulatorControl::new(rtc, watchdog)));

    SIMULATOR_CONTROL.init(Some(simulator_control));

    match Platform::init(spawner).await {
        Ok(platform_ctx) => {
            if let Err(e) = main_task::<Platform>(spawner, platform_ctx).await {
                error!("Main task error: {:?}", e);
            }
        }
        Err(e) => {
            error!("Platform init error: {:?}", e);
        }
    }
}
