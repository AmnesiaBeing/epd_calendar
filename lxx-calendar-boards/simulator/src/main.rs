use embassy_executor::Spawner;
use embedded_hal_mock::eh1::{delay::NoopDelay, digital::no_pin::NoPin, spi::no_spi::NoSpi};
use epd_yrd0750ryf665f60::yrd0750ryf665f60::Epd7in5;
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use simulated_rtc::SimulatedRtc;

pub mod drivers;

struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = simulated_wdt::SimulatedWdt;

    type EpdDevice = Epd7in5<NoSpi, NoPin, NoPin, NoPin, NoopDelay>;

    type AudioDevice = drivers::SimulatorBuzzer;

    type LEDDevice = NoLED;

    type RtcDevice = SimulatedRtc;

    type WifiDevice = NoWifi;

    type NetworkStack = drivers::TunTapNetwork;

    type BatteryDevice = NoBattery;

    type ButtonDevice = drivers::SimulatorButton;

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>> {
        let wdt = simulated_wdt::SimulatedWdt::new(30000);
        simulated_wdt::start_watchdog(&spawner, 30000);

        let epd = drivers::init_epd().await;

        let audio = drivers::SimulatorBuzzer;
        let mut rtc = simulated_rtc::SimulatedRtc::new();
        rtc.initialize().await.ok();

        let wifi = NoWifi::new();
        let network = drivers::TunTapNetwork::new(spawner)?;
        let led = NoLED::new();
        let battery = NoBattery::new(3700, false, false);

        let button = drivers::SimulatorButton::new();

        Ok(PlatformContext {
            sys_watch_dog: wdt,
            epd,
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
        info!("Simulator platform reset");
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
