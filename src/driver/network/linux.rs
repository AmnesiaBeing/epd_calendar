use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, StackResources, StaticConfigV4};
use embassy_net_tuntap::TunTapDevice;
use static_cell::StaticCell;

use crate::common::error::Result;
use crate::driver::network::NetworkDriver;
use crate::{common::error::AppError, driver::lcg};

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, TunTapDevice>) -> ! {
    runner.run().await
}

pub struct LinuxNetworkDriver {
    pub stack: Option<Stack<'static>>,
}

impl LinuxNetworkDriver {
    pub fn new() -> Self {
        Self { stack: None }
    }
}

impl NetworkDriver for LinuxNetworkDriver {
    async fn initialize(&mut self, spawner: &Spawner) -> Result<()> {
        let device = TunTapDevice::new("tap99")
            .map_err(|e| AppError::NetworkStackInitFailed(format!("TAP device failed: {:?}", e)))?;

        let config = Config::ipv4_static(StaticConfigV4 {
            address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
            dns_servers: heapless::Vec::from_slice(&[Ipv4Address::new(223, 5, 5, 5)]).unwrap(),
            gateway: Some(Ipv4Address::new(192, 168, 69, 100)),
        });

        // Generate random seed
        let mut lcg = lcg::Lcg::new();
        let seed = lcg.next();

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

        self.stack = Some(stack);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.stack.as_ref().map(|s| s.is_link_up()).unwrap_or(false)
    }

    async fn connect(&mut self) -> Result<()> {
        todo!()
    }

    // fn get_stack(&self) -> Option<&Stack> {
    //     todo!()
    // }
}
