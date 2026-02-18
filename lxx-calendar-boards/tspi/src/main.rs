#![no_main]

use embassy_executor::Spawner;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use linux_embedded_hal::{SpidevDevice, SysfsPin};
use lxx_calendar_common::*;
use simulated_wdt::SimulatedWdt;

pub use embassy_executor::main as platform_main;

pub struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = SimulatedWdt;

    type EpdDevice = SpidevDevice;

    async fn init(spawner: Spawner) -> PlatformContext<Self> {
        let epd_busy = init_gpio(101, linux_embedded_hal::sysfs_gpio::Direction::In).unwrap();
        let epd_dc = init_gpio(102, linux_embedded_hal::sysfs_gpio::Direction::Out).unwrap();
        let epd_rst = init_gpio(97, linux_embedded_hal::sysfs_gpio::Direction::Out).unwrap();

        let mut spi = SpidevDevice::open("/dev/spidev3.0").unwrap();

        let mut delay = linux_embedded_hal::Delay;
        let mut epd = Epd7in5::new(&mut spi, epd_busy, epd_dc, epd_rst, &mut delay).unwrap();

        let wdt = SimulatedWdt::new(5000);
        simulated_wdt::start_watchdog(&spawner, 5000);

        PlatformContext {
            sys_watch_dog: wdt,
            epd: spi,
        }
    }

    fn sys_reset() {
        info!("TSPI platform reset");
    }

    fn sys_stop() {
        info!("TSPI platform stop");
    }
}

fn init_gpio(
    pin: u64,
    direction: linux_embedded_hal::sysfs_gpio::Direction,
) -> Result<SysfsPin, linux_embedded_hal::sysfs_gpio::Error> {
    let gpio = SysfsPin::new(pin);
    gpio.export()?;

    while !gpio.is_exported() {}

    gpio.set_direction(direction)?;

    // 输出引脚默认置高
    if direction == linux_embedded_hal::sysfs_gpio::Direction::Out {
        gpio.set_value(1)?;
    }

    Ok(gpio)
}

#[platform_main]
async fn main(spawner: embassy_executor::Spawner) {
    let platform_ctx = Platform::init(spawner).await;
    if let Err(e) = main_task::<Platform>(spawner, platform_ctx).await {
        error!("Main task error: {:?}", e);
    }
}
