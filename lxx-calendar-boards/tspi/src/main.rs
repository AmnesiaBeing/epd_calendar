use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use linux_embedded_hal::{SpidevDevice, SysfsPin};
use lxx_calendar_common::platform::PlatformTrait;
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use simulator::{HttpServer, SimulatedRtc, SimulatedWdt, SimulatorControl};
use static_cell::StaticCell;
use std::sync::Arc;
use std::thread;

pub mod drivers;

use crate::drivers::{LinuxBuzzer, LinuxWifi, TspiButton, TspiLED, TunTapNetwork};

static SIMULATOR_CONTROL: StaticCell<Option<Arc<SimulatorControl>>> = StaticCell::new();
static EVENT_CHANNEL: StaticCell<Channel<CriticalSectionRawMutex, SystemEvent, 10>> =
    StaticCell::new();

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
        if let Some(ref ctrl) = *control {
            let port = std::env::var("SIMULATOR_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080);

            let ctrl_clone = Arc::clone(ctrl);
            thread::spawn(move || {
                HttpServer::new(ctrl_clone, port).run();
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
        })
    }

    fn sys_reset() {
        info!("TSPI platform reset");
    }

    fn init_logger() {
        env_logger::init();
    }

    fn init_heap() {}
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    Platform::init_heap();
    Platform::init_logger();

    let event_channel = EVENT_CHANNEL.init(Channel::new());
    let event_sender = event_channel.sender();

    let rtc = SimulatedRtc::new();
    let watchdog = SimulatedWdt::new(5000);

    let simulator_control = Arc::new(SimulatorControl::new(rtc, watchdog, event_sender));

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
