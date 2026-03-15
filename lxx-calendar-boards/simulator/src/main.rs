use embassy_executor::Spawner;
use embedded_hal_mock::eh1::{delay::NoopDelay, digital::no_pin::NoPin, spi::no_spi::NoSpi};
use epd_yrd0750ryf665f60::yrd0750ryf665f60::Epd7in5;
use lxx_calendar_common::traits::platform::{RtcMemoryData, WakeupSource};
use lxx_calendar_common::*;
use simulator::{
    HttpServer, SimulatedBLE, SimulatedFlash, SimulatedRtc, SimulatedWdt, SimulatorButton,
    SimulatorControl, SimulatorSleepManager,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::thread;
use tokio::sync::{Mutex as TokioMutex, Notify};

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
        // 实际唤醒源从 RTC 内存读取，由 main 循环管理
        WakeupSource::PowerOn
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    Platform::init_heap();
    Platform::init_logger();

    let port = std::env::var("SIMULATOR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    info!("Starting simulator on port {}", port);

    // 创建共享的 RTC 内存（Deep Sleep 后保留）
    let rtc_memory = Arc::new(TokioMutex::new(RtcMemoryData::new()));
    let wakeup_notify = Arc::new(Notify::new());

    // 创建睡眠管理器
    let sleep_manager = SimulatorSleepManager::new(rtc_memory.clone(), wakeup_notify.clone());

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
        ble_instance.set_external_wakeup_flag(rtc_sleep_state.get_condvar());

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

        // 获取唤醒源
        let wakeup_source = {
            match rtc_memory.try_lock() {
                Ok(mem) => {
                    if mem.is_valid() {
                        mem.wakeup_source
                    } else {
                        WakeupSource::PowerOn
                    }
                }
                Err(_) => WakeupSource::PowerOn,
            }
        };
        info!("Wakeup source: {:?}", wakeup_source);

        // 初始化平台
        match Platform::init(spawner).await {
            Ok(platform_ctx) => {
                // 执行主任务
                if let Err(e) = run_main_task_with_sleep(
                    spawner,
                    platform_ctx,
                    sleep_manager.clone(),
                    wakeup_source,
                )
                .await
                {
                    error!("Main task error: {:?}", e);
                }
            }
            Err(e) => {
                error!("Platform init error: {:?}", e);
            }
        }

        info!("=== Simulator Deep Sleep cycle ended, restarting ===");
    }
}

/// 带睡眠管理的主任务
async fn run_main_task_with_sleep(
    spawner: Spawner,
    platform_ctx: PlatformContext<Platform>,
    mut sleep_manager: SimulatorSleepManager,
    wakeup_source: WakeupSource,
) -> SystemResult<()> {
    use lxx_calendar_common::traits::platform::SleepMode;

    info!("lxx-calendar starting... (wakeup: {:?})", wakeup_source);

    // 调用原始 main_task
    if let Err(e) = lxx_calendar_core::main_task::<Platform>(spawner, platform_ctx).await {
        error!("Main task error: {:?}", e);
        return Err(e);
    }

    // 计算下次唤醒时间（60 秒后）
    let next_wakeup = embassy_time::Duration::from_secs(60);

    // 进入 Deep Sleep
    info!("Entering deep sleep for {} seconds", next_wakeup.as_secs());

    match sleep_manager.sleep(SleepMode::DeepSleep, next_wakeup).await {
        Ok(_) => info!("Deep sleep ended"),
        Err(e) => error!("Sleep error: {:?}", e),
    }

    // Deep Sleep 后返回，外层循环会重启

    Ok(())
}
