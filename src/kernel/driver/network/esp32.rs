//! ESP32 网络驱动（RAII 锁 + STA/AP 模式 + 按需休眠）
//! 核心特性：
//! 1. 初始化非阻塞，持有网络外设
//! 2. RAII 锁管理网络使用，自动唤醒/休眠
//! 3. STA 模式：使用计数归0后延迟休眠，新请求可取消休眠
//! 4. AP 模式：配网专用，超时自动关闭

use core::sync::atomic::{AtomicUsize, Ordering};

use embassy_executor::{Spawner, task};
use embassy_net::{self, Config, Runner, Stack, StackResources};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::peripherals::WIFI;
use esp_radio::wifi::{AccessPointConfig, ClientConfig, ModeConfig, WifiController, WifiDevice};
use esp_radio::{self, Controller as RadioController};
use static_cell::StaticCell;

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::driver::network::{NetworkDriver, NetworkMode, StackGuard};
use crate::platform::Platform;
use crate::platform::esp32::Esp32Platform;

// ========== 静态资源管理 ==========
/// Radio 控制器静态实例（全局唯一）
static RADIO_CTRL: StaticCell<RadioController> = StaticCell::new();
/// 网络栈资源静态实例
static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
/// Radio 初始化标记（避免重复初始化）
static RADIO_INITIALIZED: AtomicUsize = AtomicUsize::new(0);

// ========== 网络驱动核心结构体 ==========
pub struct EspNetworkDriver {
    // 核心资源
    stack: Option<Stack<'static>>,
    controller: WifiController<'static>,
    _wifi_peripheral: WIFI<'static>, // 持有外设所有权，避免被释放

    // 状态管理
    is_initialized: bool,
    current_mode: GlobalMutex<NetworkMode>,
    pub(super) usage_count: AtomicUsize, // 网络使用计数（原子量，支持多任务）
    wifi_config: GlobalMutex<WifiConfig>, // STA 配置

    // 休眠/超时配置
    sta_sleep_delay: Duration, // STA 休眠延迟（默认 5 秒）
    sta_sleep_timer: GlobalMutex<Option<Timer>>,
    ap_timeout: Duration, // AP 配网超时（默认 5 分钟）
    ap_timeout_timer: GlobalMutex<Option<Timer>>,
}

impl EspNetworkDriver {
    // ========== 初始化相关 ==========
    /// 创建驱动实例（轻量、非阻塞）
    /// 仅持有 WiFi 外设，不执行任何耗时操作
    pub fn create(wifi_peripheral: WIFI, default_wifi_config: WifiConfig) -> Self {
        Self {
            stack: None,
            controller: unsafe { core::mem::zeroed() }, // 占位，init 时初始化
            _wifi_peripheral: wifi_peripheral,
            is_initialized: false,
            current_mode: Mutex::new(NetworkMode::Sleeping),
            usage_count: AtomicUsize::new(0),
            wifi_config: Mutex::new(default_wifi_config),
            sta_sleep_delay: Duration::from_secs(5),
            sta_sleep_timer: Mutex::new(None),
            ap_timeout: Duration::from_secs(300), // 5 分钟
            ap_timeout_timer: Mutex::new(None),
        }
    }

    /// 异步初始化驱动（非阻塞）
    /// 初始化 Radio、创建控制器、启动网络任务
    pub async fn init(&mut self, spawner: &Spawner) -> Result<()> {
        if self.is_initialized {
            return Ok(());
        }

        // 1. 初始化 Radio 控制器（全局唯一）
        let radio_ctrl = if RADIO_INITIALIZED
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            RADIO_CTRL.init(esp_radio::init().map_err(|_| AppError::NetworkRadioInitFailed)?)
        } else {
            RADIO_CTRL.get().ok_or(AppError::NetworkRadioInitFailed)?
        };

        // 2. 创建 WiFi 控制器和设备
        let (controller, interfaces) = esp_radio::wifi::new(
            radio_ctrl,
            core::mem::take(&mut self._wifi_peripheral),
            Default::default(),
        )
        .map_err(|_| AppError::NetworkControllerCreateFailed)?;
        self.controller = controller;
        let device = interfaces.sta;

        // 3. 初始化网络栈
        let config = Config::dhcpv4(Default::default());
        let seed = getrandom::u64().map_err(|_| AppError::NetworkRadioInitFailed)?;
        let resources = STACK_RESOURCES.init(StackResources::new());
        let (stack, runner) = embassy_net::new(device, config, resources, seed);
        self.stack = Some(stack);

        // 4. 启动网络任务（embassy-net 核心任务）
        spawner
            .spawn(net_task(runner))
            .map_err(|_| AppError::NetworkTaskSpawnFailed)?;

        // 5. 启动休眠监控任务
        spawner
            .spawn(self.sta_sleep_monitor_task())
            .map_err(|_| AppError::NetworkTaskSpawnFailed)?;

        // 6. 初始状态：休眠
        *self.current_mode.lock().await = NetworkMode::Sleeping;
        self.controller.set_power_save(PowerSave::Max).ok();
        self.is_initialized = true;

        log::info!("ESP32 network driver initialized (sleeping)");
        Ok(())
    }

    // ========== RAII 锁获取（上层核心接口） ==========
    /// 获取网络栈锁（自动唤醒 STA 模式）
    /// 上层通过此方法获取 Stack 引用，释放时自动管理休眠
    pub async fn acquire_stack(&self) -> Result<StackGuard<'_>> {
        if !self.is_initialized {
            return Err(AppError::NetworkNotInitialized);
        }

        // 1. 使用计数 +1
        let prev_count = self.usage_count.fetch_add(1, Ordering::AcqRel);
        log::debug!(
            "Network stack acquired, usage count: {} -> {}",
            prev_count,
            prev_count + 1
        );

        // 2. 加锁修改模式，保证线程安全
        let mut mode_guard = self.current_mode.lock().await;
        let stack = self
            .stack
            .as_ref()
            .ok_or(AppError::NetworkStackAcquireFailed)?;

        match *mode_guard {
            NetworkMode::Sleeping => {
                // 休眠状态：唤醒 STA
                self.wake_sta().await?;
                *mode_guard = NetworkMode::StaActive;
            }
            NetworkMode::StaActive => {
                // STA 活跃：取消休眠延迟
                let mut timer_guard = self.sta_sleep_timer.lock().await;
                *timer_guard = None;
            }
            NetworkMode::ApActive => {
                // AP 活跃：先关闭 AP，再唤醒 STA
                self.stop_ap().await?;
                self.wake_sta().await?;
                *mode_guard = NetworkMode::StaActive;
            }
        }

        // 3. 返回 RAII 锁
        Ok(StackGuard {
            driver: self,
            stack,
        })
    }

    // ========== STA 模式核心逻辑 ==========
    /// 唤醒 STA 模式（连接 WiFi）
    async fn wake_sta(&self) -> Result<()> {
        let wifi_config = self.wifi_config.lock().await;

        // 1. 关闭省电模式
        self.controller.set_power_save(PowerSave::None).ok();

        // 2. 配置并启动 STA
        let client_config = ClientConfig::default()
            .with_ssid(wifi_config.ssid.as_str().into())
            .with_password(wifi_config.password.as_str().into());
        self.controller
            .set_config(&ModeConfig::Client(client_config))
            .ok();
        self.controller
            .start()
            .map_err(|_| AppError::NetworkStaWakeFailed)?;
        self.controller
            .connect()
            .map_err(|_| AppError::NetworkStaWakeFailed)?;

        // 3. 等待链路 UP（超时 30 秒）
        let stack = self.stack.as_ref().ok_or(AppError::NetworkStaWakeFailed)?;
        embassy_time::with_timeout(Duration::from_secs(30), async {
            loop {
                if stack.is_link_up() {
                    break;
                }
                Timer::after(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(|_| AppError::NetworkStaWakeFailed)?;

        log::info!("STA mode wake up, connected to: {}", wifi_config.ssid);
        Ok(())
    }

    /// 休眠 STA 模式（保留 Stack，关闭射频）
    async fn sleep_sta(&self) {
        // 1. 停止 WiFi，开启最大省电
        self.controller.stop().ok();
        self.controller.set_power_save(PowerSave::Max).ok();

        // 2. 更新模式
        let mut mode_guard = self.current_mode.lock().await;
        *mode_guard = NetworkMode::Sleeping;

        log::info!("STA mode sleep (Stack retained)");
    }

    /// STA 休眠监控任务（独立运行）
    #[task]
    async fn sta_sleep_monitor_task(&self) -> ! {
        loop {
            // 检查休眠计时器
            let mut timer_guard = self.sta_sleep_timer.lock().await;
            if let Some(timer) = timer_guard.take() {
                // 等待延迟结束
                timer.await;

                // 检查使用计数：0 则休眠，>0 则重置
                if self.usage_count.load(Ordering::Acquire) == 0 {
                    self.sleep_sta().await;
                } else {
                    log::debug!("New network usage detected, cancel sleep");
                }
            }
            drop(timer_guard);

            Timer::after(Duration::from_millis(100)).await;
        }
    }

    // ========== AP 配网模式核心逻辑 ==========
    /// 启动 AP 配网模式（超时自动关闭）
    pub async fn start_ap_for_config(&self) -> Result<()> {
        if !self.is_initialized {
            return Err(AppError::NetworkNotInitialized);
        }

        // 1. 加锁修改状态
        let mut mode_guard = self.current_mode.lock().await;
        if *mode_guard == NetworkMode::StaActive {
            self.sleep_sta().await;
        }

        // 2. 配置 AP（配网专用：开放热点，仅 1 个连接）
        let ap_config = AccessPointConfig::default()
            .with_ssid("ESP32-Config".into())
            .with_channel(1)
            .with_max_connections(1);
        self.controller
            .set_config(&ModeConfig::AccessPoint(ap_config))
            .map_err(|_| AppError::NetworkApStartFailed)?;
        self.controller
            .start()
            .map_err(|_| AppError::NetworkApStartFailed)?;

        // 3. 启动 AP 超时计时器
        let mut ap_timer_guard = self.ap_timeout_timer.lock().await;
        *ap_timer_guard = Some(Timer::after(self.ap_timeout));

        // 4. 启动 AP 超时监控任务
        embassy_executor::spawn(self.ap_timeout_monitor_task()).ok();

        // 5. 更新模式
        *mode_guard = NetworkMode::ApActive;
        log::info!(
            "AP config mode started (timeout: {}min)",
            self.ap_timeout.as_secs() / 60
        );
        Ok(())
    }

    /// 完成 AP 配网（更新 WiFi 配置，关闭 AP）
    pub async fn finish_ap_config(&self, new_ssid: &str, new_password: &str) -> Result<()> {
        // 1. 更新 WiFi 配置
        let mut wifi_config_guard = self.wifi_config.lock().await;
        *wifi_config_guard = WifiConfig::new(new_ssid, new_password)?;

        // 2. 停止 AP，恢复休眠
        self.stop_ap().await?;
        let mut mode_guard = self.current_mode.lock().await;
        *mode_guard = NetworkMode::Sleeping;

        log::info!("AP config finished, new SSID: {}", new_ssid);
        Ok(())
    }

    /// 停止 AP 模式
    async fn stop_ap(&self) -> Result<()> {
        self.controller
            .stop()
            .map_err(|_| AppError::NetworkApStopFailed)?;
        log::info!("AP mode stopped");
        Ok(())
    }

    /// AP 超时监控任务
    #[task]
    async fn ap_timeout_monitor_task(&self) {
        // 等待超时
        let mut ap_timer_guard = self.ap_timeout_timer.lock().await;
        let Some(timer) = ap_timer_guard.take() else {
            return;
        };
        drop(ap_timer_guard);
        timer.await;

        // 超时后关闭 AP
        let mut mode_guard = self.current_mode.lock().await;
        if *mode_guard == NetworkMode::ApActive {
            let _ = self.stop_ap().await;
            *mode_guard = NetworkMode::Sleeping;
            log::warn!(
                "AP config timeout ({}min), auto stopped",
                self.ap_timeout.as_secs() / 60
            );
        }
    }

    // ========== 配置修改接口 ==========
    /// 修改 STA 休眠延迟
    pub fn set_sta_sleep_delay(&mut self, delay: Duration) {
        self.sta_sleep_delay = delay;
        log::debug!("STA sleep delay set to {}s", delay.as_secs());
    }

    /// 修改 AP 配网超时
    pub fn set_ap_timeout(&mut self, timeout: Duration) {
        self.ap_timeout = timeout;
        log::debug!("AP config timeout set to {}min", timeout.as_secs() / 60);
    }
}

impl EspNetworkDriver {
    /// 启动 STA 休眠延迟
    pub(super) fn start_sleep_delay(&self) {
        let mut timer_guard = self.sta_sleep_timer.lock_blocking();
        *timer_guard = Some(Timer::after(self.sta_sleep_delay));
        log::debug!(
            "STA sleep delay started ({}s)",
            self.sta_sleep_delay.as_secs()
        );
    }
}

// ========== embassy-net 核心任务 ==========
#[task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await;
}

impl NetworkDriver for EspNetworkDriver {
    type P = Esp32Platform;
    fn new(peripherals: &<Self::P as Platform>::Peripherals, spawner: &Spawner) -> Result<Self> {
        Self::new(peripherals, spawner)
    }

    async fn acquire_stack(&self) -> Result<StackGuard<'_>> {
        self.acquire_stack().await
    }

    async fn start_ap_for_config(&self) -> Result<()> {
        self.start_ap_for_config().await
    }

    async fn finish_ap_config(&self, ssid: &str, password: &str) -> Result<()> {
        self.finish_ap_config(ssid, password).await
    }

    async fn current_mode(&self) -> NetworkMode {
        *self.current_mode.lock().await
    }

    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
