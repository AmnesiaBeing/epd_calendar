// src/kernel/driver/network/linux.rs

//! Linux平台网络驱动实现
//!
//! 提供Linux平台的网络连接功能，基于TUN/TAP设备实现虚拟网络接口

use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, StackResources, StaticConfigV4};
use embassy_net_tuntap::TunTapDevice;
use static_cell::StaticCell;

use crate::common::error::AppError;
use crate::common::error::Result;
use crate::kernel::driver::network::NetworkDriver;

/// 网络任务异步函数
///
/// # 参数
/// - `runner`: 网络栈运行器
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, TunTapDevice>) -> ! {
    runner.run().await
}

/// Linux网络驱动结构体
///
/// 管理Linux平台的虚拟网络接口连接
pub struct LinuxNetworkDriver {
    /// 网络栈实例
    pub stack: Option<Stack<'static>>,
}

impl LinuxNetworkDriver {
    /// 创建新的Linux网络驱动实例
    ///
    /// # 返回值
    /// - `Self`: 驱动实例
    pub fn new() -> Self {
        Self { stack: None }
    }
}

impl NetworkDriver for LinuxNetworkDriver {
    /// 初始化网络栈
    ///
    /// # 参数
    /// - `spawner`: 异步任务生成器
    ///
    /// # 返回值
    /// - `Result<()>`: 初始化结果
    async fn initialize(&mut self, spawner: &Spawner) -> Result<()> {
        let device = TunTapDevice::new("tap99").map_err(|e| {
            log::error!("Failed to create TAP device: {:?}", e);
            AppError::NetworkStackInitFailed
        })?;

        let config = Config::ipv4_static(StaticConfigV4 {
            address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
            dns_servers: heapless_08::Vec::from_slice(&[Ipv4Address::new(223, 5, 5, 5)]).unwrap(),
            gateway: Some(Ipv4Address::new(192, 168, 69, 100)),
        });

        // Generate random seed
        let seed = getrandom::u64().map_err(|_| AppError::NetworkStackInitFailed)?;

        // Init network stack
        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        let (stack, runner) = embassy_net::new(
            device,
            config,
            RESOURCES.init(StackResources::new()),
            seed as u64,
        );

        // Launch network task
        if let Err(e) = spawner.spawn(net_task(runner)) {
            log::error!("Failed to spawn net task: {}", e);
            return Err(AppError::NetworkError);
        }

        stack.wait_config_up().await;

        self.stack = Some(stack);
        Ok(())
    }

    /// 检查网络连接状态
    ///
    /// # 返回值
    /// - `bool`: 是否已连接
    fn is_connected(&self) -> bool {
        self.stack.as_ref().map(|s| s.is_link_up()).unwrap_or(false)
    }

    /// 建立网络连接（Linux平台暂未实现）
    ///
    /// # 返回值
    /// - `Result<()>`: 连接结果
    async fn connect(&mut self, _ssid: &str, _password: &str) -> Result<()> {
        todo!()
    }

    /// 获取网络栈实例
    ///
    /// # 返回值
    /// - `Option<&embassy_net::Stack<'_>>`: 网络栈引用
    fn get_stack(&self) -> Option<&embassy_net::Stack<'_>> {
        self.stack.as_ref()
    }
}
