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
    stack: Stack<'static>,
}

impl TunTapNetwork {
    pub fn new(
        spawner: embassy_executor::Spawner,
    ) -> Result<Self, lxx_calendar_common::types::error::NetworkError> {
        let device = TunTapDevice::new(TUNTAP_NAME)
            .map_err(|_| lxx_calendar_common::types::error::NetworkError::NotConnected)?;

        let seed = getrandom::u64()
            .map_err(|_| lxx_calendar_common::types::error::NetworkError::Unknown)?;

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

        Ok(Self { stack: *stack_ref })
    }
}

impl NetworkStack for TunTapNetwork {
    type Error = lxx_calendar_common::types::error::NetworkError;

    fn is_link_up(&self) -> bool {
        true
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        embassy_time::with_timeout(Duration::from_secs(10), self.stack.wait_link_up())
            .await
            .map_err(|_| lxx_calendar_common::types::error::NetworkError::Timeout)
    }

    fn get_stack(&self) -> Option<&embassy_net::Stack<'static>> {
        Some(&self.stack)
    }
}

#[embassy_executor::task(pool_size = 1)]
async fn net_task(mut runner: Runner<'static, TunTapDevice>) {
    runner.run().await
}
