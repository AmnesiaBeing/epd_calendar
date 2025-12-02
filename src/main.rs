// src/main.rs
#![cfg_attr(feature = "embedded_esp", no_std)]
#![cfg_attr(feature = "embedded_esp", no_main)]

#[cfg(feature = "embedded_esp")]
use core::prelude::v1::*;

extern crate alloc;

use embassy_executor::Spawner;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use static_cell::StaticCell;

mod assets;
mod common;
mod driver;
mod render;
mod service;
mod tasks;

use crate::common::GlobalMutex;
use crate::common::system_state::{SYSTEM_STATE, SystemState};
use crate::driver::display::DefaultDisplayDriver;
use crate::driver::network::{DefaultNetworkDriver, NetworkDriver};
use crate::driver::ntp_source::SntpSource;
use crate::driver::power::DefaultPowerMonitor;
use crate::driver::sensor::DefaultSensorDriver;
use crate::driver::storage::DefaultConfigStorage;
use crate::driver::time_source::DefaultTimeSource;
use crate::render::RenderEngine;
use crate::service::{ConfigService, QuoteService, TimeService, WeatherService};
use crate::tasks::{display_task, quote_task, status_task, time_task, weather_task};

// 全局状态管理
static NETWORK_DRIVER: StaticCell<GlobalMutex<DefaultNetworkDriver>> = StaticCell::new();
static SENSOR_DRIVER: StaticCell<GlobalMutex<DefaultSensorDriver>> = StaticCell::new();
static POWER_MONITOR: StaticCell<GlobalMutex<DefaultPowerMonitor>> = StaticCell::new();

static WEATHER_SERVICE: StaticCell<GlobalMutex<WeatherService>> = StaticCell::new();
static TIME_SERVICE: StaticCell<GlobalMutex<TimeService>> = StaticCell::new();
static CONFIG_SERVICE: StaticCell<GlobalMutex<ConfigService>> = StaticCell::new();

static RENDER_ENGINE: StaticCell<GlobalMutex<RenderEngine>> = StaticCell::new();

#[cfg(feature = "embedded_esp")]
esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    cold_start(&spawner).await;
}

#[cfg(feature = "embedded_esp")]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    cold_start(&spawner).await;

    loop {}
}

/// 冷启动初始化
async fn cold_start(spawner: &Spawner) {
    // 初始化日志系统
    init_logging().await;
    log::info!("EPD Calendar starting...");

    log::info!("Cold start initializing system...");

    #[cfg(feature = "embedded_esp")]
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // 初始化存储驱动与配置服务
    let storage_driver = DefaultConfigStorage::new(&peripherals)
        .map_err(|e| {
            log::error!("Failed to create config storage driver: {}", e);
            e
        })
        .unwrap();

    let mut config_service = ConfigService::new(storage_driver);

    // 初始化硬件驱动
    let display_driver = match DefaultDisplayDriver::new(&peripherals) {
        Ok(driver) => driver,
        Err(e) => {
            log::error!("Failed to create display driver: {}", e);
            return;
        }
    };

    #[cfg(feature = "embedded_esp")]
    let mut network_driver = DefaultNetworkDriver::new(&peripherals).unwrap();
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    let mut network_driver = DefaultNetworkDriver::new();

    match network_driver.initialize(spawner).await {
        Ok(driver) => driver,
        Err(e) => {
            log::error!("Failed to initialize network driver: {}", e);
            return;
        }
    };

    // 初始化网络驱动
    let network_driver_mutex = NETWORK_DRIVER.init(Mutex::new(network_driver));

    // 初始化时间服务
    let ntp_time_source = SntpSource::new(network_driver_mutex);
    #[cfg(feature = "embedded_esp")]
    let time_source = DefaultTimeSource::new(&peripherals);
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    let time_source = DefaultTimeSource::new();
    let time_service = TimeService::new(time_source, ntp_time_source);

    // 初始化其他驱动和服务
    // let sensor_driver = SENSOR_DRIVER.init(Mutex::new(DefaultSensorDriver::new()));
    let power_monitor = POWER_MONITOR.init(Mutex::new(DefaultPowerMonitor::new()));

    // 初始化渲染引擎
    let render_engine = RenderEngine::new(display_driver);

    // 启动显示任务
    spawner.spawn(display_task(render_engine)).unwrap();

    // 启动其他任务
    spawner.spawn(quote_task(QuoteService::new())).unwrap();
    spawner
        .spawn(status_task(power_monitor, network_driver_mutex))
        .unwrap();
    spawner.spawn(time_task(time_service)).unwrap();
    spawner
        .spawn(weather_task(WeatherService::new(network_driver_mutex)))
        .unwrap();

    log::info!("EPD Calendar started successfully");
}

/// 初始化日志系统
async fn init_logging() {
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    {
        env_logger::init();
        log::info!("Initialized env_logger for simulator/embedded_linux");
    }

    #[cfg(feature = "embedded_esp")]
    {
        rtt_target::rtt_init_print!();
        log::info!("Initializing logger for ESP32");
    }
}

/// 主循环 - 处理系统级事件
async fn main_loop() {
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

/// 记录系统健康状态
async fn log_system_health() {
    log::debug!("System health check");

    // 这里可以添加更多的系统健康检查
    // 例如：内存使用情况、任务状态等

    log::debug!("System health check completed");
}
