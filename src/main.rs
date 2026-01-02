// src/main.rs
//! EPD日历应用主入口模块

#![cfg_attr(feature = "embedded_esp", no_std)]
#![cfg_attr(feature = "embedded_esp", no_main)]
#![cfg_attr(
    feature = "embedded_esp",
    deny(
        clippy::mem_forget,
        reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
    )
)]
#![cfg_attr(feature = "embedded_esp", deny(clippy::large_stack_frames))]

extern crate alloc;

use embassy_executor::Spawner;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
#[cfg(feature = "embedded_esp")]
use esp_hal::clock::CpuClock;
#[cfg(feature = "embedded_esp")]
use esp_hal::timer::timg::TimerGroup;
use static_cell::StaticCell;

mod assets;
mod common;
mod kernel;
mod tasks;

use crate::common::GlobalMutex;
use crate::common::error::Result;
use crate::kernel::data::DataSourceRegistry;
use crate::kernel::data::generic_scheduler_task;
use crate::kernel::data::sources::config::ConfigDataSource;
use crate::kernel::data::sources::time::TimeDataSource;
use crate::kernel::data::sources::weather::WeatherDataSource;
use crate::kernel::driver::display::DefaultDisplayDriver;
use crate::kernel::driver::network::{DefaultNetworkDriver, NetworkDriver};
use crate::kernel::driver::ntp_source::SntpService;
use crate::kernel::driver::power::DefaultPowerDriver;
use crate::kernel::driver::sensor::DefaultSensorDriver;
use crate::kernel::driver::storage::DefaultConfigStorageDriver;
use crate::kernel::driver::time_driver::DefaultTimeDriver;
use crate::kernel::system::api::DefaultSystemApi;
use crate::tasks::display_task;

/// 全局驱动状态管理
static DISPLAY_DRIVER: StaticCell<GlobalMutex<DefaultDisplayDriver>> = StaticCell::new();
static NETWORK_DRIVER: StaticCell<GlobalMutex<DefaultNetworkDriver>> = StaticCell::new();
static TIME_DRIVER: StaticCell<GlobalMutex<DefaultTimeDriver>> = StaticCell::new();
static SYSTEM_API: StaticCell<GlobalMutex<DefaultSystemApi>> = StaticCell::new();

/// 全局数据源管理
static CONFIG_SOURCE: StaticCell<GlobalMutex<ConfigDataSource>> = StaticCell::new();
static TIME_SOURCE: StaticCell<GlobalMutex<TimeDataSource>> = StaticCell::new();
static WEATHER_SOURCE: StaticCell<GlobalMutex<WeatherDataSource>> = StaticCell::new();

#[cfg(feature = "embedded_esp")]
esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(feature = "embedded_esp")]
use panic_rtt_target as _;

#[cfg(feature = "embedded_esp")]
use rtt_target::rprintln;

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_executor::main as platform_main;

#[cfg(feature = "embedded_esp")]
use esp_rtos::main as platform_main;

/// 应用程序主入口
///
/// # 参数
/// - `spawner`: 任务生成器，用于启动异步任务
///
/// # 功能
/// - 冷启动初始化系统组件
/// - 启动所有后台任务
/// - 进入主循环进行系统监控
#[platform_main]
async fn main(spawner: Spawner) {
    // 冷启动初始化
    let _ = cold_start(&spawner).await;

    // 主循环
    let mut last_system_check = Instant::now();
    const SYSTEM_CHECK_INTERVAL: Duration = Duration::from_secs(60);

    log::info!("Entering main loop");

    loop {
        // 定期检查系统状态
        if last_system_check.elapsed() > SYSTEM_CHECK_INTERVAL {
            log_system_health().await;
            last_system_check = Instant::now();
        }

        // 主循环休眠，让任务运行
        Timer::after(Duration::from_secs(30)).await;
    }
}

/// 冷启动初始化系统组件
///
/// # 参数
/// - `spawner`: 任务生成器
///
/// # 初始化步骤
/// 1. 初始化日志系统
/// 2. 初始化存储驱动和配置服务
/// 3. 初始化网络驱动
/// 4. 初始化时间源和SNTP服务
/// 5. 初始化显示驱动和渲染引擎
/// 6. 启动所有后台任务
async fn cold_start(spawner: &Spawner) -> Result<()> {
    // 初始化日志系统
    init_logging().await;
    log::info!("EPD Calendar starting...");

    log::info!("Cold start initializing system...");

    #[cfg(feature = "embedded_esp")]
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    #[cfg(feature = "embedded_esp")]
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);

    #[cfg(feature = "embedded_esp")]
    let timg0 = TimerGroup::new(peripherals.TIMG0);

    #[cfg(feature = "embedded_esp")]
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    #[cfg(feature = "embedded_esp")]
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    // 初始化存储驱动与配置服务
    #[cfg(feature = "embedded_esp")]
    let storage_driver = DefaultConfigStorageDriver::new(peripherals.FLASH).unwrap();
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    let storage_driver = DefaultConfigStorageDriver::new("flash.bin", 4096).unwrap();

    #[cfg(feature = "embedded_esp")]
    let mut network_driver = DefaultNetworkDriver::new(&peripherals).unwrap();
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    let mut network_driver = DefaultNetworkDriver::new();

    network_driver.initialize(spawner).await?;

    // 初始化网络驱动
    let network_driver_mutex = NETWORK_DRIVER.init(Mutex::new(network_driver));

    // 初始化时间驱动
    #[cfg(feature = "embedded_esp")]
    let time_driver = DefaultTimeDriver::new(&peripherals);
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    let time_driver = DefaultTimeDriver::new();
    let time_driver_mutex = TIME_DRIVER.init(Mutex::new(time_driver));

    // 初始化SNTP更新时间驱动
    SntpService::initialize(spawner, network_driver_mutex, time_driver_mutex);

    // 初始化其他驱动和服务
    #[cfg(feature = "embedded_esp")]
    let power_driver = DefaultPowerDriver::new();
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    let power_driver = DefaultPowerDriver::new();

    // 初始化显示驱动
    #[cfg(feature = "embedded_esp")]
    let display_driver = DefaultDisplayDriver::new(&peripherals).await.unwrap();
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    let display_driver = DefaultDisplayDriver::new().await.unwrap();

    let display_driver_mutex = DISPLAY_DRIVER.init(Mutex::new(display_driver));

    let data_source_registry = DataSourceRegistry::init();

    let system_api = SYSTEM_API.init(Mutex::new(DefaultSystemApi::new(
        power_driver,
        network_driver_mutex,
        storage_driver,
        DefaultSensorDriver::new(),
        display_driver_mutex,
    )));

    // 初始化并启动显示任务
    spawner
        .spawn(display_task(display_driver_mutex, data_source_registry))
        .unwrap();

    // 设置数据源注册表到系统API
    let mut system_api_guard = system_api.lock().await;
    system_api_guard.set_data_source_registry(data_source_registry);
    drop(system_api_guard);

    // 注册配置数据源
    let config_source_mutex =
        CONFIG_SOURCE.init(Mutex::new(ConfigDataSource::new(system_api).await?));
    data_source_registry
        .lock()
        .await
        .register_source(config_source_mutex, system_api)
        .await?;

    // 注册时间数据源
    let time_source_mutex = TIME_SOURCE.init(Mutex::new(TimeDataSource::new(time_driver_mutex)?));
    data_source_registry
        .lock()
        .await
        .register_source(time_source_mutex, system_api)
        .await?;

    // 注册天气数据源
    let weather_source_mutex =
        WEATHER_SOURCE.init(Mutex::new(WeatherDataSource::new(system_api).await?));
    data_source_registry
        .lock()
        .await
        .register_source(weather_source_mutex, system_api)
        .await?;

    spawner
        .spawn(generic_scheduler_task(data_source_registry, system_api))
        .unwrap();

    log::info!("EPD Calendar started successfully");

    Ok(())
}

/// 初始化日志系统
///
/// # 功能
/// - 根据平台配置不同的日志系统
/// - 嵌入式ESP32使用RTT日志
/// - Linux模拟器使用env_logger
async fn init_logging() {
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    {
        env_logger::init();
        log::info!("Initialized env_logger for simulator/embedded_linux");
    }

    #[cfg(feature = "embedded_esp")]
    {
        rtt_target::rtt_init_print!();
        esp_println::logger::init_logger_from_env();
        log::info!("Initializing logger for ESP32");
    }
}

/// 记录系统健康状态
///
/// # 功能
/// - 定期记录系统运行状态
/// - 可用于监控系统健康状况
async fn log_system_health() {
    log::debug!("System health check");

    // 这里可以添加更多的系统健康检查
    // 例如：内存使用情况、任务状态等

    log::debug!("System health check completed");
}
