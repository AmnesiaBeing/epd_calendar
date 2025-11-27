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
// use core::display_manager::{DisplayManager, RefreshMode};

use crate::app_core::display_manager::{DisplayManager, RefreshMode};
use crate::common::system_state::{SYSTEM_STATE, SystemState};
use crate::common::types::SystemConfig;
use crate::driver::display::DefaultDisplayDriver;
use crate::driver::time_source::DefaultTimeSource;
use crate::render::RenderEngine;
use crate::service::{ConfigService, TimeService};

static DISPLAY_MANAGER: StaticCell<Mutex<ThreadModeRawMutex, DisplayManager>> = StaticCell::new();
static DISPLAY_DATA: StaticCell<Mutex<ThreadModeRawMutex, DisplayData>> = StaticCell::new();
static RENDER_ENGINE: StaticCell<Mutex<ThreadModeRawMutex, RenderEngine>> = StaticCell::new();
static CONFIG: StaticCell<Mutex<ThreadModeRawMutex, SystemConfig>> = StaticCell::new();
static TIME_SERVICE: StaticCell<Mutex<ThreadModeRawMutex, TimeService>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // 初始化日志系统
    init_logging().await;
    log::info!("EPD Calendar starting...");

    // 检查是否已经初始化（从休眠唤醒）
    if let Some(_) = SYSTEM_STATE.try_get() {
        // 从休眠中恢复
        log::info!("System already initialized, resuming from sleep");
    } else {
        // 冷启动
        log::info!("Cold start initializing system...");

        // 初始化存储驱动与配置服务（从存储读取配对信息）
        let storage_driver = driver::storage::create_default_storage().await.unwrap();
        let mut config_service = ConfigService::new(storage_driver);
        let system_config = config_service.load_config().await.unwrap_or_default();

        // 初始化硬件驱动
        let display_driver: DefaultDisplayDriver = DefaultDisplayDriver::new().await.unwrap();
        let time_source = DefaultTimeSource::new();
        // let network_driver = init_network_driver().await;
        // let power_monitor = init_power_monitor().await;

        // 初始化服务层（传入配置）
        let time_service = TimeService::new(
            time_source,
            system_config.time_format_24h,
            system_config.temperature_celsius,
        );
        // let weather_service = service::weather_service::WeatherService::new(
        //     network_driver.clone(),
        //     system_config.temperature_celsius,
        // );
        // let quote_service = QuoteService::new();

        // 初始化核心管理器
        let display_manager = DisplayManager::new(LayoutConfig::MAX_PARTIAL_REFRESHES);

        // 初始化渲染引擎
        let render_engine = RenderEngine::new(display_driver);

        // 初始化共享状态
        let display_manager = DISPLAY_MANAGER.init(Mutex::new(display_manager));
        let display_data = DISPLAY_DATA.init(Mutex::new(DisplayData::default()));
        let render_engine = RENDER_ENGINE.init(Mutex::new(render_engine));
        let config = CONFIG.init(Mutex::new(system_config));
        let time_service = TIME_SERVICE.init(Mutex::new(time_service));

        // 注册显示区域
        register_display_regions(&display_manager).await;

        // 执行初始全局显示设置
        if let Err(e) = initial_display_setup(
            &display_manager,
            &display_data,
            &render_engine,
            &time_service,
            // &weather_service,
            // &quote_service,
        )
        .await
        {
            log::error!("Initial display setup failed: {}", e);
            return;
        }

        // 启动所有任务
        spawn_tasks(
            spawner,
            display_manager,
            display_data,
            render_engine,
            time_service,
            config,
            // weather_service,
            // quote_service,
            // power_monitor,
            // network_driver,
        )
        .await;

        // 初始化全局系统状态
        let _ = SYSTEM_STATE.init(SystemState::default());

        log::info!("EPD Calendar started successfully");
    }

    // 10. 主循环
    main_loop().await;
}

/// 初始化日志系统
async fn init_logging() {
    #[cfg(any(feature = "simulator", feature = "embedded_linux"))]
    env_logger::init();

    #[cfg(feature = "embedded_esp")]
    log_to_defmt::init();
}

/// 初始化网络驱动
// async fn init_network_driver() -> Result<impl driver::network::NetworkDriver> {
//     driver::network::MockNetworkDriver::new() // 先用模拟实现
// }

/// 初始化电源监控
// async fn init_power_monitor() -> Result<impl driver::power::PowerMonitor> {
//     driver::power::MockPowerMonitor::new() // 先用模拟实现
// }

/// 注册显示区域
async fn register_display_regions(display_manager: &Mutex<ThreadModeRawMutex, DisplayManager>) {
    let mut dm = display_manager.lock().await;

    dm.register_region("time", LayoutConfig::TIME_REGION);
    dm.register_region("date", LayoutConfig::DATE_REGION);
    dm.register_region("weather", LayoutConfig::WEATHER_REGION);
    dm.register_region("quote", LayoutConfig::QUOTE_REGION);
    dm.register_region("status", LayoutConfig::STATUS_REGION);
}

/// 初始显示设置
async fn initial_display_setup(
    display_manager: &Mutex<ThreadModeRawMutex, DisplayManager>,
    display_data: &Mutex<ThreadModeRawMutex, DisplayData>,
    render_engine: &Mutex<ThreadModeRawMutex, RenderEngine>,
    time_service: &Mutex<ThreadModeRawMutex, TimeService>,
    // weather_service: &WeatherService<impl driver::network::NetworkDriver>,
    // quote_service: &QuoteService,
) -> Result<()> {
    log::info!("Performing initial global display setup");

    // 强制全局刷新模式
    {
        let mut dm = display_manager.lock().await;
        dm.set_refresh_mode(RefreshMode::Global);
    }

    // 获取初始数据
    let time_service = time_service.lock().await;
    let initial_time = time_service.get_current_time().await?;
    // let initial_weather = weather_service.get_weather().await.unwrap_or_default();
    // let initial_quote = quote_service.get_random_quote().await?;

    // 更新显示数据
    {
        let mut data = display_data.lock().await;
        data.time = initial_time;
        // data.weather = initial_weather;
        // data.quote = initial_quote;
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
    spawner: Spawner,
    display_manager: &'static Mutex<ThreadModeRawMutex, DisplayManager>,
    display_data: &'static Mutex<ThreadModeRawMutex, DisplayData>,
    render_engine: &'static Mutex<ThreadModeRawMutex, RenderEngine>,
    time_service: &'static Mutex<ThreadModeRawMutex, TimeService>,
    config: &'static Mutex<ThreadModeRawMutex, SystemConfig>,
    // weather_service: service::weather_service::WeatherService<impl driver::network::NetworkDriver>,
    // quote_service: service::quote_service::QuoteService,
    // power_monitor: impl driver::power::PowerMonitor,
    // network_driver: impl driver::network::NetworkDriver,
) {
    // 时间任务
    if let Err(e) = spawner.spawn(tasks::time_task::time_task(
        display_manager,
        display_data,
        time_service,
        config,
    )) {
        log::error!("Failed to spawn time task: {}", e);
    }

    // 天气任务
    // if let Err(e) = spawner.spawn(tasks::weather_task::weather_task(
    //     display_manager.clone(),
    //     display_data.clone(),
    //     weather_service,
    // )) {
    //     log::error!("Failed to spawn weather task: {}", e);
    // }

    // 格言任务
    // if let Err(e) = spawner.spawn(tasks::quote_task::quote_task(
    //     display_manager,
    //     display_data,
    //     // quote_service,
    // )) {
    //     log::error!("Failed to spawn quote task: {}", e);
    // }

    // 状态任务
    if let Err(e) = spawner.spawn(tasks::status_task::status_task(
        display_manager,
        display_data,
        // power_monitor,
        // network_driver,
    )) {
        log::error!("Failed to spawn status task: {}", e);
    }

    // 显示刷新任务
    if let Err(e) = spawner.spawn(tasks::display_refresh_task::display_refresh_task(
        display_manager,
        display_data,
        render_engine,
    )) {
        log::error!("Failed to spawn display refresh task: {}", e);
    }

    log::info!("All tasks spawned successfully");
}

/// 主循环 - 处理系统级事件
async fn main_loop() {
    let mut last_system_check = Instant::now();

    loop {
        // 每分钟检查一次系统状态
        if last_system_check.elapsed() > Duration::from_secs(60) {
            check_system_health().await;
            last_system_check = Instant::now();
        }

        // 主循环休眠，让任务运行
        Timer::after(Duration::from_secs(30)).await;
    }
}

/// 系统健康检查
async fn check_system_health() {
    // 记录系统运行状态
    log::debug!("System health check");
}
