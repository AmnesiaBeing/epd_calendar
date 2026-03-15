use embassy_executor::Spawner;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use linux_embedded_hal::{SpidevDevice, SysfsPin};
use lxx_calendar_common::platform::PlatformTrait;
use lxx_calendar_common::traits::platform::WakeupSource;
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

pub mod drivers;

use crate::drivers::{LinuxBuzzer, LinuxWifi, TspiButton, TspiLED, TunTapNetwork};

#[embassy_executor::task]
async fn embassy_main_task(spawner: Spawner) {
    // 获取唤醒源
    let wakeup_source = Platform::get_wakeup_source();
    info!("TSPi wakeup source: {:?}", wakeup_source);

    // 初始化平台
    match Platform::init(spawner).await {
        Ok(platform_ctx) => {
            // 执行主任务
            if let Err(e) = main_task::<Platform>(spawner, platform_ctx).await {
                error!("Main task error: {:?}", e);
            }
        }
        Err(e) => {
            error!("Platform init error: {:?}", e);
        }
    }

    // 进入 Deep Sleep（60 秒后唤醒）
    let next_wakeup = embassy_time::Duration::from_secs(60);
    info!("TSPi entering deep sleep for {} seconds", next_wakeup.as_secs());

    let wakeup_source = Platform::deep_sleep(next_wakeup).await;
    info!("TSPi deep sleep ended, wakeup source: {:?}", wakeup_source);
}

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
        WakeupSource::PowerOn
    }

    async fn deep_sleep(duration: embassy_time::Duration) -> WakeupSource {
        // 使用 tokio 等待模拟 Deep Sleep
        tokio::time::sleep(std::time::Duration::from_millis(duration.as_millis())).await;
        WakeupSource::RtcTimer
    }
}

#[tokio::main]
async fn main() {
    Platform::init_heap();
    Platform::init_logger();

    let rtc = SimulatedRtc::new();
    let watchdog = SimulatedWdt::new(5000);

    let simulator_control = Arc::new(Mutex::new(SimulatorControl::new(rtc, watchdog)));

    SIMULATOR_CONTROL.init(Some(simulator_control));

    // Deep Sleep 循环：TSPi 逻辑重启
    loop {
        info!("=== TSPi Deep Sleep cycle starting ===");

        // 在 tokio 任务中运行 embassy 执行器
        let embassy_handle = tokio::task::spawn_blocking(move || {
            static EXECUTOR: StaticCell<embassy_executor::Executor> = StaticCell::new();
            let executor = EXECUTOR.init(embassy_executor::Executor::new());
            executor.run(|spawner| {
                // 使用 task 宏定义的任务
                spawner.spawn(embassy_main_task(spawner)).ok();
            });
        });

        // 等待 embassy 任务完成（Deep Sleep 后返回）
        let _ = embassy_handle.await;

        info!("=== TSPi Deep Sleep cycle ended, restarting ===");
    }
}
