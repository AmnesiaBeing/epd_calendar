use embassy_executor::Spawner;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use linux_embedded_hal::{SpidevDevice, SysfsPin};
use lxx_calendar_common::platform::PlatformTrait;
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;

pub mod drivers;

use crate::drivers::{LinuxBuzzer, LinuxNetwork, LinuxWifi, TspiLED, TunTapNetwork};

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
    type WatchdogDevice = simulated_wdt::SimulatedWdt;

    type EpdDevice = SpidevDevice;

    type AudioDevice = LinuxBuzzer;

    type LEDDevice = TspiLED;

    type RtcDevice = simulated_rtc::SimulatedRtc;

    type WifiDevice = LinuxWifi;

    type NetworkStack = LinuxNetwork;

    type BatteryDevice = NoBattery;

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>> {
        let epd_busy = init_gpio(101, linux_embedded_hal::sysfs_gpio::Direction::In).unwrap();
        let epd_dc = init_gpio(102, linux_embedded_hal::sysfs_gpio::Direction::Out).unwrap();
        let epd_rst = init_gpio(97, linux_embedded_hal::sysfs_gpio::Direction::Out).unwrap();

        let mut spi = SpidevDevice::open("/dev/spidev3.0").unwrap();

        let mut delay = linux_embedded_hal::Delay;
        let _epd = Epd7in5::new(&mut spi, epd_busy, epd_dc, epd_rst, &mut delay)
            .await
            .unwrap();

        let wdt = simulated_wdt::SimulatedWdt::new(5000);
        simulated_wdt::start_watchdog(&spawner, 5000);

        let audio = LinuxBuzzer;

        let mut rtc = simulated_rtc::SimulatedRtc::new();
        rtc.initialize().await.ok();

        let wifi = LinuxWifi::new();
        let network = TunTapNetwork::new(spawner)?;
        let led = TspiLED;
        let battery = NoBattery::new(3700, false, false);

        Ok(PlatformContext {
            sys_watch_dog: wdt,
            epd: spi,
            audio,
            rtc,
            wifi,
            network,
            led,
            battery,
        })
    }

    fn sys_reset() {
        info!("TSPI platform reset");
    }

    fn sys_stop() {
        info!("TSPI platform stop");
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
