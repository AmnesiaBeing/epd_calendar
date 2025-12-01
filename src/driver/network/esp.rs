use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_net::{Config, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::peripherals::{SW_INTERRUPT, TIMG0, WIFI};
use esp_hal::timer::timg::TimerGroup;
use esp_radio::Controller;
use esp_radio::wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice};
use static_cell::StaticCell;

use crate::common::error::AppError;
use crate::common::error::Result;
use crate::driver::network::NetworkDriver;

static TIMG0_GROUP: StaticCell<TimerGroup<TIMG0>> = StaticCell::new();
static SW_INT_CTRL: StaticCell<SoftwareInterruptControl> = StaticCell::new();
static ESP_RADIO_CTRL: StaticCell<Controller> = StaticCell::new();

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await
}

pub struct EspNetworkDriver {
    controller: Option<WifiController<'static>>,
    device: Option<WifiDevice<'static>>,
    stack: Option<Stack<'static>>,
    is_initialized: bool,
}

impl EspNetworkDriver {
    // pub fn new(timg0: TIMG0, sw_interrupt: SW_INTERRUPT, wifi: WIFI) -> Result<Self> {
    pub fn new(wifi: WIFI<'static>) -> Result<Self> {
        // 初始化ESP-RADIO
        let esp_radio_ctrl =
            ESP_RADIO_CTRL.init(esp_radio::init().map_err(|_| AppError::NetworkStackInitFailed)?);

        // 创建WiFi控制器和接口
        let (controller, interfaces) =
            esp_radio::wifi::new(esp_radio_ctrl, wifi, Default::default())
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

const WIFI_SSID: &str = "WIFI_SSID";
const WIFI_PASSWORD: &str = "WIFI_PASSWORD";

impl NetworkDriver for EspNetworkDriver {
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

        // 生成随机种子（与Linux版本保持一致）
        let mut lcg = crate::driver::lcg::Lcg::new();
        let seed = lcg.next();

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

        controller.set_config(&client_config);
        controller
            .start()
            .map_err(|e| AppError::WifiConnectionFailed)?;
        controller.connect();

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

    fn is_connected(&self) -> bool {
        self.stack.as_ref().map(|s| s.is_link_up()).unwrap_or(false)
    }

    fn get_stack(&self) -> Option<&Stack> {
        self.stack.as_ref()
    }
}
