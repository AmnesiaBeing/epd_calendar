use embassy_net::{Config, Runner, Stack, StackResources, StaticConfigV4, Ipv4Cidr, Ipv4Address};
use embassy_net_tuntap::TunTapDevice;
use embassy_time::Duration;
use heapless_08::Vec as Vec08;
use lxx_calendar_common::NetworkStack;
use lxx_calendar_common::*;
use static_cell::StaticCell;

const TUNTAP_NAME: &str = "tap99";

static STACK_RESOURCE: StaticCell<StackResources<3>> = StaticCell::new();
static STACK: StaticCell<Stack<'static>> = StaticCell::new();

pub struct TunTapNetwork {
    stack: Option<Stack<'static>>,
}

impl TunTapNetwork {
    pub fn new(
        spawner: embassy_executor::Spawner,
    ) -> Result<Self, lxx_calendar_common::types::error::NetworkError> {
        let device = match TunTapDevice::new(TUNTAP_NAME) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("Failed to create TunTap device: {:?}, network unavailable", e);
                return Ok(Self { stack: None });
            }
        };

        let mut buf = [0u8; 8];
        let _ = getrandom::getrandom(&mut buf);
        let seed = u64::from_le_bytes(buf);

        let config = Config::ipv4_static(StaticConfigV4 {
            address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 101), 24),
            gateway: Some(Ipv4Address::new(192, 168, 69, 100)),
            dns_servers: {
                let mut dns = Vec08::new();
                let _ = dns.push(Ipv4Address::new(223, 5, 5, 5));     // 阿里 DNS
                let _ = dns.push(Ipv4Address::new(119, 29, 29, 29)); // 腾讯 DNS
                dns
            },
        });

        let (stack, runner) = embassy_net::new(
            device,
            config,
            STACK_RESOURCE.init(StackResources::<3>::new()),
            seed,
        );

        let stack_ref = STACK.init(stack);

        info!("Network stack created with static IP 192.168.69.101, gateway 192.168.69.100, DNS: 223.5.5.5, 119.29.29.29");
        spawner.spawn(net_task(runner)).ok();

        Ok(Self { stack: Some(*stack_ref) })
    }
}

impl NetworkStack for TunTapNetwork {
    type Error = lxx_calendar_common::types::error::NetworkError;

    fn is_link_up(&self) -> bool {
        self.stack.is_some()
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        match &self.stack {
            Some(stack) => {
                info!("Waiting for network link up...");
                embassy_time::with_timeout(Duration::from_secs(10), stack.wait_link_up())
                    .await
                    .map_err(|_| lxx_calendar_common::types::error::NetworkError::Timeout)?;
                info!("Network link is up");
                Ok(())
            }
            None => Err(lxx_calendar_common::types::error::NetworkError::NotConnected),
        }
    }

    fn get_stack(&self) -> Option<&embassy_net::Stack<'static>> {
        self.stack.as_ref()
    }
}

#[embassy_executor::task(pool_size = 1)]
async fn net_task(mut runner: Runner<'static, TunTapDevice>) {
    runner.run().await
}