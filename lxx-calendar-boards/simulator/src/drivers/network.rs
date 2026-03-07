use embassy_net::{Config, Runner, Stack, StackResources};
use embassy_net_tuntap::TunTapDevice;
use embassy_time::Duration;
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

        let config = Config::dhcpv4(Default::default());

        let (stack, runner) = embassy_net::new(
            device,
            config,
            STACK_RESOURCE.init(StackResources::<3>::new()),
            seed,
        );

        let stack_ref = STACK.init(stack);

        debug!("Network stack created");
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
                embassy_time::with_timeout(Duration::from_secs(10), stack.wait_link_up())
                    .await
                    .map_err(|_| lxx_calendar_common::types::error::NetworkError::Timeout)
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
