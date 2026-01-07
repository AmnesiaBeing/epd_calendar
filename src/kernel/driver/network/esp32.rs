use embassy_executor::Spawner;
use embassy_net::{self, Config, Stack, StackResources};
use embassy_sync::once_lock::OnceLock;
use embedded_svc::wifi::asynch::Wifi;
use enumset::EnumSet;
use esp_radio::wifi::{WifiController, WifiDevice};
use static_cell::StaticCell;

use crate::common::error::{AppError, Result};
use crate::kernel::driver::network::NetworkDriver;
use crate::platform::Platform;
use crate::platform::esp32::Esp32Platform;

// ========== 全局静态资源（核心：所有资源均为 'static） ==========
/// ESP-RADIO控制器（全局唯一，'static）
static ESP_RADIO_CTRL: StaticCell<esp_radio::Controller> = StaticCell::new();
/// 网络栈实例（全局唯一，'static）
static NET_STACK: StaticCell<Stack<'static>> = StaticCell::new();
/// WiFi控制器（全局唯一，'static）
static WIFI_CONTROLLER: OnceLock<WifiController<'static>> = OnceLock::new();

// ========== 网络驱动核心结构体（持有静态资源的引用） ==========
/// 无需生命周期参数：所有资源均为 'static
pub struct Esp32NetworkDriver {
    /// 标记是否初始化完成（避免重复初始化）
    initialized: bool,
}

/// 网络任务异步函数
///
/// # 参数
/// - `runner`: 网络栈运行器
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await
}

impl NetworkDriver for Esp32NetworkDriver {
    type P = Esp32Platform;

    async fn new(
        peripherals: &mut <Self::P as Platform>::Peripherals,
        spawner: &Spawner,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        // 1. 初始化全局Radio控制器（'static）
        let esp_radio_ctrl =
            ESP_RADIO_CTRL.init(esp_radio::init().map_err(|_| AppError::NetworkStackInitFailed)?);

        // 3. 创建WiFi控制器和设备（绑定到静态Radio/外设，生命周期 'static）
        let (controller, interfaces) = esp_radio::wifi::new(
            esp_radio_ctrl,
            unsafe { core::mem::transmute(peripherals.WIFI.clone_unchecked()) }, // 拓展生命周期
            Default::default(),
        )
        .map_err(|_| AppError::NetworkStackInitFailed)?;

        let sta_device = interfaces.sta;

        // 5. 存储WiFi控制器到全局静态区（'static）
        WIFI_CONTROLLER
            .init(controller)
            .map_err(|_| AppError::NetworkStackInitFailed)?;

        let config = Config::dhcpv4(Default::default());
        let seed = getrandom::u64().map_err(|_| AppError::NetworkStackInitFailed)?;

        // 初始化网络栈资源
        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        let resources = RESOURCES.init(StackResources::new());

        // 创建网络栈
        let (stack, runner) = embassy_net::new(sta_device, config, resources, seed as u64);

        // 6. 初始化全局网络栈（'static）
        // let stack_resources = NET_STACK_RESOURCES.init(StackResources::new());
        NET_STACK.init(stack);

        // 7. 生成网络任务（参数为 'static Runner，符合要求）
        spawner
            .spawn(net_task(runner))
            .map_err(|_| AppError::NetworkTaskSpawnFailed)?;

        Ok(Self { initialized: true })
    }
}

/// 实现Wifi trait（所有资源均为全局静态 'static）
impl Wifi for Esp32NetworkDriver {
    type Error = AppError;

    async fn get_capabilities(&self) -> Result<EnumSet<embedded_svc::wifi::Capability>> {
        todo!()
    }

    async fn get_configuration(&self) -> Result<embedded_svc::wifi::Configuration> {
        // 从全局控制器获取配置
        // let ctrl = WIFI_CONTROLLER.get().await;
        // // 示例：替换为实际的配置获取逻辑
        // Ok(embedded_svc::wifi::Configuration::Client(
        //     embedded_svc::wifi::ClientConfiguration {
        //         ssid: "".into(),
        //         password: "".into(),
        //         ..Default::default()
        //     },
        // ))
        todo!()
    }

    async fn set_configuration(&mut self, _conf: &embedded_svc::wifi::Configuration) -> Result<()> {
        // let mut ctrl = WIFI_CONTROLLER.get().await;
        // 根据配置设置WiFi模式
        // match conf {
        //     embedded_svc::wifi::Configuration::Client(client_conf) => {
        //         let mode_config = ModeConfig::Client(ClientConfig {
        //             ssid: client_conf.ssid.clone().into(),
        //             password: client_conf.password.clone().into(),
        //             ..Default::default()
        //         });
        //         ctrl.set_mode(mode_config)
        //             .await
        //             .map_err(|_| AppError::NetworkConfigFailed)?;
        //     }
        //     embedded_svc::wifi::Configuration::AccessPoint(ap_conf) => {
        //         let mode_config = ModeConfig::AccessPoint(AccessPointConfig {
        //             ssid: ap_conf.ssid.clone().into(),
        //             password: ap_conf.password.clone().into(),
        //             ..Default::default()
        //         });
        //         ctrl.set_mode(mode_config)
        //             .await
        //             .map_err(|_| AppError::NetworkConfigFailed)?;
        //     }
        //     _ => return Err(AppError::NetworkUnsupportedConfig),
        // }
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        // let mut ctrl = WIFI_CONTROLLER.get().await;
        // ctrl.start()
        //     .await
        //     .map_err(|_| AppError::NetworkStartFailed)?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        // let mut ctrl = WIFI_CONTROLLER.get().await;
        // ctrl.stop().await.map_err(|_| AppError::NetworkStopFailed)?;
        Ok(())
    }

    async fn connect(&mut self) -> Result<()> {
        // let mut ctrl = WIFI_CONTROLLER.get().await;
        // ctrl.connect()
        //     .await
        //     .map_err(|_| AppError::NetworkConnectFailed)?;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // let mut ctrl = WIFI_CONTROLLER.get().await;
        // ctrl.disconnect()
        //     .await
        //     .map_err(|_| AppError::NetworkDisconnectFailed)?;
        Ok(())
    }

    async fn is_started(&self) -> Result<bool> {
        // let ctrl = WIFI_CONTROLLER.get().await;
        // Ok(ctrl.is_started().await)
        Ok(false)
    }

    async fn is_connected(&self) -> Result<bool> {
        // let ctrl = WIFI_CONTROLLER.get().await;
        // Ok(ctrl.is_connected().await)
        Ok(false)
    }

    async fn scan_n<const N: usize>(
        &mut self,
    ) -> Result<(
        heapless_08::Vec<embedded_svc::wifi::AccessPointInfo, N>,
        usize,
    )> {
        // let mut ctrl = WIFI_CONTROLLER
        //     .get_mut()
        //     .ok_or(AppError::NetworkControllerNotInitialized)?;
        // let mut results = heapless_08::Vec::new();
        // let count = ctrl.scan().await.map_err(|_| AppError::NetworkScanFailed)?;

        // // 示例：简化的扫描结果处理（需根据实际API调整）
        // Ok((results, count))
        todo!()
    }
}
