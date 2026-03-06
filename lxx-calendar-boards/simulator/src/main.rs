use embassy_executor::Spawner;
use embedded_hal_mock::eh1::{delay::NoopDelay, digital::no_pin::NoPin, spi::no_spi::NoSpi};
use epd_yrd0750ryf665f60::yrd0750ryf665f60::Epd7in5;
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use simulator::{
    HttpServer, SimulatedBLE, SimulatedRtc, SimulatedWdt, SimulatorButton, SimulatorControl,
};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;

pub mod drivers;

struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = SimulatedWdt;

    type EpdDevice = Epd7in5<NoSpi, NoPin, NoPin, NoPin, NoopDelay>;

    type AudioDevice = drivers::SimulatorBuzzer;

    type LEDDevice = NoLED;

    type RtcDevice = SimulatedRtc;

    type WifiDevice = NoWifi;

    type NetworkStack = drivers::TunTapNetwork;

    type BatteryDevice = NoBattery;

    type ButtonDevice = SimulatorButton;

    type BLEDevice = SimulatedBLE;

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>> {
        let wdt = SimulatedWdt::new(30000);
        simulator::start_watchdog(&spawner, 30000);

        let epd = drivers::init_epd().await;

        let audio = drivers::SimulatorBuzzer;
        let mut rtc = SimulatedRtc::new();
        rtc.initialize().await.ok();

        let wifi = NoWifi::new();
        let network = drivers::TunTapNetwork::new(spawner)?;
        let led = NoLED::new();
        let battery = NoBattery::new(3700, false, false);

        let button = SimulatorButton::new();
        let ble = SimulatedBLE::new();

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
            ble,
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

    let port = std::env::var("SIMULATOR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    // 创建共享的 BLE 实例
    let mut ble_instance = SimulatedBLE::new();
    
    // 创建 Rtc 并获取 wakeup flag
    let rtc_for_button = SimulatedRtc::new();
    let rtc_wakeup = rtc_for_button.get_wakeup_flag();
    
    // 让 BLE 使用与 RTC 相同的 wakeup flag
    ble_instance.set_external_wakeup_flag(rtc_wakeup.clone());
    
    let ble_for_http = ble_instance.clone();
    let ble_for_app = ble_instance;
    
    // 创建 Button 并设置 wakeup flag
    let mut button_for_http = SimulatorButton::new();
    button_for_http.set_wakeup_flag(rtc_wakeup);

    let control = Arc::new(Mutex::new(SimulatorControl::new(
        SimulatedRtc::new(),
        SimulatedWdt::new(30000),
    )));

    let ble = Arc::new(Mutex::new(ble_for_http));
    let button = Arc::new(Mutex::new(button_for_http));

    let ctrl_clone = Arc::clone(&control);
    let ble_clone = Arc::clone(&ble);
    let btn_clone = Arc::clone(&button);
    thread::spawn(move || {
        HttpServer::new(ctrl_clone, ble_clone, btn_clone, port).run();
    });

    match Platform::init(spawner).await {
        Ok(mut platform_ctx) => {
            // 使用共享的 BLE 实例
            platform_ctx.ble = ble_for_app;
            if let Err(e) = main_task::<Platform>(spawner, platform_ctx).await {
                error!("Main task error: {:?}", e);
            }
        }
        Err(e) => {
            error!("Platform init error: {:?}", e);
        }
    }
}
