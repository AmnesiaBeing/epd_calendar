// src/main.rs
use embassy_executor::Spawner;
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

mod app_core;
mod assets;
mod common;
mod driver;
mod render;
mod service;
mod tasks;

use common::config::LayoutConfig;
use common::error::Result;
use common::types::DisplayData;
use static_cell::StaticCell;

use crate::app_core::display_manager::{DisplayManager, RefreshMode};
use crate::common::system_state::{SYSTEM_STATE, SystemState};
use crate::common::types::SystemConfig;
use crate::driver::display::DefaultDisplayDriver;
use crate::driver::network::NetworkDriver;
use crate::driver::power::DefaultPowerMonitor;
use crate::driver::sensor::DefaultSensorDriver;
use crate::driver::storage::DefaultStorageDriver;
use crate::driver::time_source::{DefaultNtpTimeSource, DefaultTimeSource};
use crate::render::RenderEngine;
use crate::service::weather_service::WeatherService;
use crate::service::{config_service::ConfigService, time_service::TimeService};

// 全局状态管理
type GlobalMutex<T> = Mutex<ThreadModeRawMutex, T>;

static NETWORK_DRIVER: StaticCell<GlobalMutex<NetworkDriver>> = StaticCell::new();
static DISPLAY_MANAGER: StaticCell<GlobalMutex<DisplayManager>> = StaticCell::new();
static SENSOR_DRIVER: StaticCell<GlobalMutex<DefaultSensorDriver>> = StaticCell::new();
static WEATHER_SERVICE: StaticCell<GlobalMutex<WeatherService>> = StaticCell::new();
static TIME_SERVICE: StaticCell<GlobalMutex<TimeService>> = StaticCell::new();
static RENDER_ENGINE: StaticCell<GlobalMutex<RenderEngine>> = StaticCell::new();
static POWER_MONITOR: StaticCell<GlobalMutex<DefaultPowerMonitor>> = StaticCell::new();
static CONFIG_SERVICE: StaticCell<GlobalMutex<ConfigService<DefaultStorageDriver>>> =
    StaticCell::new();
static DISPLAY_DATA: StaticCell<GlobalMutex<DisplayData>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // 初始化日志系统
    init_logging().await;
    log::info!("EPD Calendar starting...");

    // 检查是否已经初始化（从休眠唤醒）
    if SYSTEM_STATE.try_get().is_some() {
        // 从休眠中恢复
        log::info!("System already initialized, resuming from sleep");
        resume_from_sleep(&spawner).await;
    } else {
        // 冷启动
        log::info!("Cold start initializing system...");
        cold_start(&spawner).await;
    }
}

/// 冷启动初始化
async fn cold_start(spawner: &Spawner) {
    // 初始化存储驱动与配置服务
    let storage_driver = match driver::storage::create_default_storage().await {
        Ok(driver) => driver,
        Err(e) => {
            log::error!("Failed to create storage driver: {}", e);
            return;
        }
    };

    let mut config_service = ConfigService::new(storage_driver);
    let system_config = match config_service.load_config().await {
        Ok(config) => config,
        Err(e) => {
            log::warn!("Failed to load config, using defaults: {}", e);
            SystemConfig::default()
        }
    };

    // 初始化硬件驱动
    let display_driver = match DefaultDisplayDriver::new().await {
        Ok(driver) => driver,
        Err(e) => {
            log::error!("Failed to create display driver: {}", e);
            return;
        }
    };

    let network_driver = match NetworkDriver::new(spawner).await {
        Ok(driver) => driver,
        Err(e) => {
            log::error!("Failed to initialize network driver: {}", e);
            return;
        }
    };

    // 初始化网络驱动
    let network_mutex = NETWORK_DRIVER.init(Mutex::new(network_driver));

    // 初始化时间服务
    let ntp_time_source = DefaultNtpTimeSource::new(network_mutex);
    let time_source = DefaultTimeSource::new(ntp_time_source);
    let time_service = TimeService::new(
        time_source,
        system_config.time_format_24h,
        system_config.temperature_celsius,
    );

    // 初始化其他驱动和服务
    let sensor_driver = SENSOR_DRIVER.init(Mutex::new(DefaultSensorDriver::new()));
    let power_monitor = POWER_MONITOR.init(Mutex::new(DefaultPowerMonitor::new()));
    let weather_service = WEATHER_SERVICE.init(Mutex::new(WeatherService::new(
        network_mutex,
        system_config.temperature_celsius,
    )));

    // 初始化核心管理器
    let display_manager = DisplayManager::new(LayoutConfig::MAX_PARTIAL_REFRESHES);

    // 初始化渲染引擎
    let render_engine = RenderEngine::new(display_driver);

    // 初始化共享状态
    let display_manager_mutex = DISPLAY_MANAGER.init(Mutex::new(display_manager));
    let display_data_mutex = DISPLAY_DATA.init(Mutex::new(DisplayData::default()));
    let render_engine_mutex = RENDER_ENGINE.init(Mutex::new(render_engine));
    let config_service_mutex = CONFIG_SERVICE.init(Mutex::new(config_service));
    let time_service_mutex = TIME_SERVICE.init(Mutex::new(time_service));

    // 注册显示区域
    register_display_regions(display_manager_mutex).await;

    // 执行初始全局显示设置
    if let Err(e) = initial_display_setup(
        display_manager_mutex,
        display_data_mutex,
        render_engine_mutex,
        time_service_mutex,
    )
    .await
    {
        log::error!("Initial display setup failed: {}", e);
        return;
    }

    // 启动所有任务
    spawn_tasks(
        spawner,
        display_manager_mutex,
        display_data_mutex,
        render_engine_mutex,
        time_service_mutex,
        config_service_mutex,
        weather_service,
        power_monitor,
        network_mutex,
        sensor_driver,
    )
    .await;

    // 初始化全局系统状态
    let _ = SYSTEM_STATE.init(SystemState::default());

    log::info!("EPD Calendar started successfully");

    // 进入主循环
    main_loop().await;
}

/// 从休眠中恢复
async fn resume_from_sleep(spawner: &Spawner) {
    log::info!("Resuming from sleep...");

    // 这里可以实现从休眠恢复的具体逻辑
    // 例如：重新初始化网络连接、更新显示数据等

    log::info!("System resumed from sleep");

    // 进入主循环
    main_loop().await;
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
        // ESP32 特定的日志初始化
        log::info!("Initializing logger for ESP32");
        // 实际的 ESP32 日志初始化代码
    }

    #[cfg(not(any(
        feature = "simulator",
        feature = "embedded_linux",
        feature = "embedded_esp"
    )))]
    {
        log::info!("No specific logger initialized - using default");
    }
}

/// 注册显示区域
async fn register_display_regions(display_manager: &'static GlobalMutex<DisplayManager>) {
    let mut dm = display_manager.lock().await;

    dm.register_region("time", LayoutConfig::TIME_REGION);
    dm.register_region("date", LayoutConfig::DATE_REGION);
    dm.register_region("weather", LayoutConfig::WEATHER_REGION);
    dm.register_region("quote", LayoutConfig::QUOTE_REGION);
    dm.register_region("status", LayoutConfig::STATUS_REGION);

    log::info!("Display regions registered successfully");
}

/// 初始显示设置
async fn initial_display_setup(
    display_manager: &'static GlobalMutex<DisplayManager>,
    display_data: &'static GlobalMutex<DisplayData<'static>>,
    render_engine: &'static GlobalMutex<RenderEngine>,
    time_service: &'static GlobalMutex<TimeService>,
) -> Result<()> {
    log::info!("Performing initial global display setup");

    // 强制全局刷新模式
    {
        let mut dm = display_manager.lock().await;
        dm.set_refresh_mode(RefreshMode::Global);
    }

    // 获取初始数据
    let initial_time = {
        let time_svc = time_service.lock().await;
        time_svc.get_current_time().await?
    };

    // 更新显示数据
    {
        let mut data = display_data.lock().await;
        data.time = initial_time;
        data.force_refresh = true;
    }

    // 执行首次全局渲染
    {
        let data = display_data.lock().await.clone();
        let mut engine = render_engine.lock().await;
        engine.render_full_display(&data).await?;
    }

    // 重置刷新模式为局部刷新
    {
        let mut dm = display_manager.lock().await;
        dm.set_refresh_mode(RefreshMode::Partial);
        dm.reset_refresh_counters();
    }

    log::info!("Initial display setup completed");
    Ok(())
}

/// 启动所有任务
async fn spawn_tasks(
    spawner: &Spawner,
    display_manager: &'static GlobalMutex<DisplayManager>,
    display_data: &'static GlobalMutex<DisplayData<'static>>,
    render_engine: &'static GlobalMutex<RenderEngine>,
    time_service: &'static GlobalMutex<TimeService>,
    config: &'static GlobalMutex<ConfigService<DefaultStorageDriver>>,
    weather_service: &'static GlobalMutex<WeatherService>,
    power_monitor: &'static GlobalMutex<DefaultPowerMonitor>,
    network_driver: &'static GlobalMutex<NetworkDriver>,
    sensor_driver: &'static GlobalMutex<DefaultSensorDriver>,
) {
    // 时间任务
    if let Err(e) = spawner.spawn(tasks::time_task::time_task(
        display_manager,
        display_data,
        time_service,
        config,
    )) {
        log::error!("Failed to spawn time task: {}", e);
    } else {
        log::info!("Time task spawned successfully");
    }

    // 天气任务
    if let Err(e) = spawner.spawn(tasks::weather_task::weather_task(
        display_manager,
        display_data,
        weather_service,
        sensor_driver,
    )) {
        log::error!("Failed to spawn weather task: {}", e);
    } else {
        log::info!("Weather task spawned successfully");
    }

    // 状态任务
    if let Err(e) = spawner.spawn(tasks::status_task::status_task(
        display_manager,
        display_data,
        power_monitor,
    )) {
        log::error!("Failed to spawn status task: {}", e);
    } else {
        log::info!("Status task spawned successfully");
    }

    // 显示刷新任务
    if let Err(e) = spawner.spawn(tasks::display_refresh_task::display_refresh_task(
        display_manager,
        display_data,
        render_engine,
    )) {
        log::error!("Failed to spawn display refresh task: {}", e);
    } else {
        log::info!("Display refresh task spawned successfully");
    }

    log::info!("All tasks spawned successfully");
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
