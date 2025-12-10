// src/main.rs
//! EPD日历应用主入口模块

#![cfg_attr(feature = "embedded_esp", no_std)]
#![cfg_attr(feature = "embedded_esp", no_main)]

#[cfg(feature = "embedded_esp")]
use core::prelude::v1::*;

extern crate alloc;

use embassy_executor::Spawner;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::draw_target::DrawTarget;

use static_cell::StaticCell;
use epd_waveshare::epd7in5_yrd0750ryf665f60::Display7in5;
use epd_waveshare::color::QuadColor;

mod assets;
mod common;
mod kernel;
// mod tasks;

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
use crate::kernel::driver::display::DisplayDriver;
use crate::kernel::render::layout::engine::{DEFAULT_ENGINE, RenderEngine};
use crate::kernel::system::api::DefaultSystemApi;

/// 全局驱动状态管理
static NETWORK_DRIVER: StaticCell<GlobalMutex<DefaultNetworkDriver>> = StaticCell::new();
static TIME_DRIVER: StaticCell<GlobalMutex<DefaultTimeDriver>> = StaticCell::new();
static SYSTEM_API: StaticCell<GlobalMutex<DefaultSystemApi>> = StaticCell::new();

/// 全局数据源管理
static CONFIG_SOURCE: StaticCell<GlobalMutex<ConfigDataSource>> = StaticCell::new();
static TIME_SOURCE: StaticCell<GlobalMutex<TimeDataSource>> = StaticCell::new();
static WEATHER_SOURCE: StaticCell<GlobalMutex<WeatherDataSource>> = StaticCell::new();

/// 全局显示驱动和渲染引擎
static DISPLAY_DRIVER: StaticCell<GlobalMutex<DefaultDisplayDriver>> = StaticCell::new();
/// 全局显示缓冲区
static DISPLAY_BUFFER: StaticCell<GlobalMutex<Display7in5>> = StaticCell::new();

#[cfg(feature = "embedded_esp")]
esp_bootloader_esp_idf::esp_app_desc!();

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
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // 初始化存储驱动与配置服务
    #[cfg(feature = "embedded_esp")]
    let storage_driver = DefaultConfigStorageDriver::new(&peripherals).unwrap();
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
    let _ = SntpService::initialize(&spawner, network_driver_mutex, time_driver_mutex);

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

    // 创建显示缓冲区
    let display_buffer = Display7in5::default();

    let data_source_registry = DataSourceRegistry::new();

    // 存储显示驱动和缓冲区到全局变量
    let display_driver_mutex = DISPLAY_DRIVER.init(GlobalMutex::new(display_driver));
    let display_buffer_mutex = DISPLAY_BUFFER.init(GlobalMutex::new(display_buffer));

    // 初始化并启动显示任务
    spawner.spawn(display_task(display_driver_mutex, display_buffer_mutex, data_source_registry)).unwrap();

    let system_api = SYSTEM_API.init(Mutex::new(DefaultSystemApi::new(
        power_driver,
        network_driver_mutex,
        storage_driver,
        DefaultSensorDriver::new(),
    )));

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
        .register_source(config_source_mutex)
        .await?;

    // 注册时间数据源
    let time_source_mutex = TIME_SOURCE.init(Mutex::new(TimeDataSource::new(time_driver_mutex)?));
    data_source_registry
        .lock()
        .await
        .register_source(time_source_mutex)
        .await?;

    // 注册天气数据源
    let weather_source_mutex =
        WEATHER_SOURCE.init(Mutex::new(WeatherDataSource::new(system_api).await?));
    data_source_registry
        .lock()
        .await
        .register_source(weather_source_mutex)
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

/// 显示任务
/// 负责渲染布局并在数据源更新时更新布局
#[embassy_executor::task]
async fn display_task(
    display_driver: &'static GlobalMutex<DefaultDisplayDriver>,
    display_buffer: &'static GlobalMutex<Display7in5>,
    data_source_registry: &'static GlobalMutex<DataSourceRegistry>,
) {
    log::info!("Display task started");
    
    // 首次渲染布局
    render_layout(display_driver, display_buffer, data_source_registry).await;
    
    // 跟踪上次渲染时间
    let mut last_render_time = embassy_time::Instant::now();
    
    // 设置数据源更新检测的间隔
    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_secs(1));
    
    loop {
        // 等待ticker触发
        ticker.next().await;
        
        // 检查数据源是否有更新
        let data_source_guard = data_source_registry.lock().await;
        let last_update_time = data_source_guard.get_last_any_updated();
        drop(data_source_guard);
        
        // 如果数据源有更新，刷新布局
        if last_update_time > last_render_time {
            log::info!("DataSource updated, refreshing layout");
            render_layout(display_driver, display_buffer, data_source_registry).await;
            last_render_time = last_update_time;
        }
    }
}

/// 渲染布局到显示屏
async fn render_layout(
    display_driver: &'static GlobalMutex<DefaultDisplayDriver>,
    display_buffer: &'static GlobalMutex<Display7in5>,
    data_source_registry: &'static GlobalMutex<DataSourceRegistry>,
) {
    log::info!("Rendering layout");
    
    let mut buffer_guard = display_buffer.lock().await;
    let data_source_guard = data_source_registry.lock().await;
    
    // 清除显示缓冲区
    buffer_guard.clear(QuadColor::White).unwrap();
    
    // 使用默认渲染引擎渲染布局到缓冲区
    if let Ok(needs_redraw) = DEFAULT_ENGINE.render_layout(&mut *buffer_guard, &data_source_guard) {
        if needs_redraw {
            log::info!("Layout rendered successfully, updating display");
            
            // 将缓冲区内容更新到显示驱动并刷新屏幕
            let mut display_guard = display_driver.lock().await;
            
            // 将缓冲区传递给显示驱动的update_frame方法
            if let Err(e) = display_guard.update_frame(buffer_guard.buffer()) {
                log::error!("Failed to update frame: {:?}", e);
                return;
            }
            
            // 调用display_frame在屏幕上实际渲染
            if let Err(e) = display_guard.display_frame() {
                log::error!("Failed to display frame: {:?}", e);
            }
        } else {
            log::info!("No redraw needed");
        }
    } else {
        log::error!("Failed to render layout");
    }
}

/// ESP32平台 panic 处理程序
#[cfg(feature = "embedded_esp")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
