use embassy_executor::Spawner;
use embedded_hal_mock::eh1::{delay::NoopDelay, digital::no_pin::NoPin, spi::no_spi::NoSpi};
use epd_yrd0750ryf665f60::yrd0750ryf665f60::Epd7in5;
use lxx_calendar_common::traits::platform::WakeupSource;
use lxx_calendar_common::*;
use simulator::{
    HttpServer, SimulatedBLE, SimulatedFlash, SimulatedRtc, SimulatedWdt, SimulatorButton,
    SimulatorControl,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use static_cell::StaticCell;
use std::thread;

pub mod drivers;

#[embassy_executor::task]
async fn embassy_main_task(spawner: Spawner) {
    // 获取唤醒源
    let wakeup_source = Platform::get_wakeup_source();
    info!("Wakeup source: {:?}", wakeup_source);

    // 初始化平台
    match Platform::init(spawner).await {
        Ok(platform_ctx) => {
            // 执行主任务（只执行一次，不进入事件循环）
            if let Err(e) = run_once_main_task(spawner, platform_ctx).await {
                error!("Main task error: {:?}", e);
            }
        }
        Err(e) => {
            error!("Platform init error: {:?}", e);
        }
    }

    // 进入 Deep Sleep（60 秒后唤醒）
    let next_wakeup = embassy_time::Duration::from_secs(60);
    info!("Entering deep sleep for {} seconds", next_wakeup.as_secs());

    let wakeup_source = Platform::deep_sleep(next_wakeup).await;
    info!("Deep sleep ended, wakeup source: {:?}", wakeup_source);
}

/// 执行一次主任务（不进入事件循环）
async fn run_once_main_task<P: lxx_calendar_common::traits::platform::PlatformTrait>(
    _spawner: embassy_executor::Spawner,
    _platform_ctx: lxx_calendar_common::traits::platform::PlatformContext<P>,
) -> lxx_calendar_common::SystemResult<()> {
    use lxx_calendar_common::*;

    info!("lxx-calendar starting...");

    // 简单实现：只显示日志，不实际执行任务
    info!("Task execution skipped for simulator");

    // 不进入事件循环，直接返回
    Ok(())
}

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

    type OTADevice = NoOTA;

    type FlashDevice = SimulatedFlash;

    async fn init(spawner: Spawner) -> SystemResult<PlatformContext<Self>> {
        info!("Platform init starting...");

        let wdt = SimulatedWdt::new(30000);
        simulator::start_watchdog(&spawner, 30000);
        info!("Watchdog started");

        let epd = drivers::init_epd().await;
        info!("EPD initialized");

        let audio = drivers::SimulatorBuzzer;

        let mut rtc = SimulatedRtc::new();
        rtc.initialize().await.ok();
        info!("RTC initialized");

        let wifi = NoWifi::new();

        let network = drivers::TunTapNetwork::new(spawner)?;
        info!("Network created");

        let led = NoLED::new();
        let battery = NoBattery::new(3700, false, false);

        let button = SimulatorButton::new();
        let ble = SimulatedBLE::new();

        let flash = SimulatedFlash::new(PathBuf::from("/tmp/simulator_flash.bin"));
        info!("Flash initialized");

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
            ota: NoOTA::new(),
            flash,
        })
    }

    fn sys_reset() {
        info!("Simulator platform reset");
    }

    fn init_logger() {
        let _ = env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .try_init();
    }

    fn init_heap() {}

    fn get_wakeup_source() -> WakeupSource {
        // 模拟器默认返回 PowerOn
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

    let port = std::env::var("SIMULATOR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    info!("Starting simulator on port {}", port);

    // 启动 HTTP 服务器（独立线程，始终保持运行）
    {
        let shared_rtc = Arc::new(StdMutex::new(SimulatedRtc::new()));
        {
            let mut rtc = shared_rtc.lock().unwrap();
            futures_executor::block_on(async {
                rtc.initialize().await.ok();
            });
            info!("Shared RTC initialized");
        }

        let mut ble_instance = SimulatedBLE::new();
        let rtc_sleep_state = {
            let rtc = shared_rtc.lock().unwrap();
            rtc.get_sleep_state()
        };
        ble_instance.set_external_wakeup_flag(rtc_sleep_state.get_flag());

        let ble_for_http = ble_instance.clone();
        let mut button_for_http = SimulatorButton::new();
        button_for_http.set_sleep_state(rtc_sleep_state.clone());

        let control = Arc::new(StdMutex::new(SimulatorControl::new_with_shared_rtc(
            Arc::clone(&shared_rtc),
            SimulatedWdt::new(30000),
        )));

        let ble = Arc::new(StdMutex::new(ble_for_http));
        let button = Arc::new(StdMutex::new(button_for_http));

        let ctrl_clone = Arc::clone(&control);
        let ble_clone = Arc::clone(&ble);
        let btn_clone = Arc::clone(&button);

        thread::spawn(move || {
            HttpServer::new(ctrl_clone, ble_clone, btn_clone, port).run();
        });
    }

    // Deep Sleep 循环：模拟器逻辑重启
    loop {
        info!("=== Simulator Deep Sleep cycle starting ===");

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

        info!("=== Simulator Deep Sleep cycle ended, restarting ===");
    }
}
