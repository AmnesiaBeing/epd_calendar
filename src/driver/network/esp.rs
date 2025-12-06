// src/driver/network/esp.rs

//! ESP32平台网络驱动实现
//! 
//! 提供ESP32平台的WiFi网络连接功能，基于esp-radio库实现

use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_net::{Config, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::peripherals::{Peripherals, WIFI};
use esp_radio::Controller;
use esp_radio::wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice};
use static_cell::StaticCell;

use crate::common::error::AppError;
use crate::common::error::Result;
use crate::driver::network::NetworkDriver;

/// ESP-RADIO控制器静态实例
static ESP_RADIO_CTRL: StaticCell<Controller> = StaticCell::new();

/// 网络任务异步函数
/// 
/// # 参数
/// - `runner`: 网络栈运行器
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await
}

/// ESP32网络驱动结构体
/// 
/// 管理ESP32平台的WiFi连接和网络栈
pub struct EspNetworkDriver {
    /// WiFi控制器实例
    controller: Option<WifiController<'static>>,
    /// WiFi设备接口
    device: Option<WifiDevice<'static>>,
    /// 网络栈实例
    stack: Option<Stack<'static>>,
    /// 初始化状态标志
    is_initialized: bool,
}

impl EspNetworkDriver {
    /// 创建新的ESP32网络驱动实例
    /// 
    /// # 参数
    /// - `peripherals`: ESP32硬件外设
    /// 
    /// # 返回值
    /// - `Result<Self>`: 驱动实例或错误
    pub fn new(peripherals: &Peripherals) -> Result<Self> {
        // 初始化ESP-RADIO
        let esp_radio_ctrl =
            ESP_RADIO_CTRL.init(esp_radio::init().map_err(|_| AppError::NetworkStackInitFailed)?);

        // 创建WiFi控制器和接口
        let (controller, interfaces) = esp_radio::wifi::new(
            esp_radio_ctrl,
            unsafe { peripherals.WIFI.clone_unchecked() },
            Default::default(),
        )
        .map_err(|_| AppError::NetworkStackInitFailed)?;

        let device = interfaces.sta;

        Ok(Self {
            controller: Some(controller),
            device: Some(device),
            stack: None,
            is_initialized: false,
        })
    }
}

/// WiFi网络SSID
const WIFI_SSID: &str = "WIFI_SSID";
/// WiFi网络密码
const WIFI_PASSWORD: &str = "WIFI_PASSWORD";

impl NetworkDriver for EspNetworkDriver {
    /// 初始化网络栈
    /// 
    /// # 参数
    /// - `spawner`: 异步任务生成器
    /// 
    /// # 返回值
    /// - `Result<()>`: 初始化结果
    async fn initialize(&mut self, spawner: &Spawner) -> Result<()> {
        if self.is_initialized {
            return Ok(());
        }

        // 取出设备用于创建网络栈
        let device = self
            .device
            .take()
            .ok_or_else(|| AppError::NetworkStackInitFailed)?;

        // 配置网络 - 使用DHCP
        let config = Config::dhcpv4(Default::default());

        let seed = getrandom::u64().map_err(|_| AppError::NetworkStackInitFailed)?;

        // 初始化网络栈资源
        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        let resources = RESOURCES.init(StackResources::new());

        // 创建网络栈
        let (stack, runner) = embassy_net::new(device, config, resources, seed as u64);

        // 启动网络任务
        spawner
            .spawn(net_task(runner))
            .map_err(|_| AppError::NetworkError)?;

        self.stack = Some(stack);
        self.is_initialized = true;

        Ok(())
    }

    /// 建立WiFi连接
    /// 
    /// # 返回值
    /// - `Result<()>`: 连接结果
    async fn connect(&mut self) -> Result<()> {
        // 使用控制器配置WiFi连接
        let controller = self
            .controller
            .as_mut()
            .ok_or_else(|| AppError::NetworkStackInitFailed)?;

        let stack = self
            .stack
            .as_ref()
            .ok_or_else(|| AppError::NetworkStackNotInitialized)?;

        // 配置WiFi客户端
        let client_config = ModeConfig::Client(
            ClientConfig::default()
                .with_ssid(WIFI_SSID.into())
                .with_password(WIFI_PASSWORD.into()),
        );

        let _ = controller.set_config(&client_config);
        controller
            .start()
            .map_err(|_| AppError::WifiConnectionFailed)?;
        let _ = controller.connect();

        // 等待网络连接
        embassy_time::with_timeout(Duration::from_secs(30), async {
            loop {
                if stack.is_link_up() {
                    break;
                }
                Timer::after(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(|_| AppError::WifiConnectionFailed)?;

        // 等待DHCP获取IP地址
        embassy_time::with_timeout(Duration::from_secs(10), async {
            loop {
                if stack.config_v4().is_some() {
                    break;
                }
                Timer::after(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(|_| AppError::DhcpFailed)?;

        Ok(())
    }

    /// 检查网络连接状态
    /// 
    /// # 返回值
    /// - `bool`: 是否已连接
    fn is_connected(&self) -> bool {
        self.stack.as_ref().map(|s| s.is_link_up()).unwrap_or(false)
    }

    /// 获取网络栈实例
    /// 
    /// # 返回值
    /// - `Option<&Stack>`: 网络栈引用
    fn get_stack(&self) -> Option<&Stack> {
        self.stack.as_ref()
    }
}