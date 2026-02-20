use embassy_net::{Runner, Stack, StackResources};
use esp_hal::rng::Rng;
use esp_radio::wifi::WifiDevice;
use lxx_calendar_common::NetworkStack;
use lxx_calendar_common::types::error::NetworkError;
use static_cell::StaticCell;

static STACK_RESOURCE: StaticCell<StackResources<3>> = StaticCell::new();
static STACK: StaticCell<Stack<'static>> = StaticCell::new();

pub struct Esp32NetworkStack {
    stack: Stack<'static>,
}

impl Esp32NetworkStack {
    pub fn new(
        spawner: embassy_executor::Spawner,
        wifi_device: &'static mut WifiDevice<'static>,
    ) -> Self {
        let rng = Rng::new();
        let seed = (rng.random() as u64) << 32 | rng.random() as u64;

        let config = embassy_net::Config::dhcpv4(Default::default());

        let (stack, runner) = embassy_net::new(
            wifi_device,
            config,
            STACK_RESOURCE.init(StackResources::<3>::new()),
            seed,
        );

        let stack_ref = STACK.init(stack);

        spawner.spawn(net_task(runner)).ok();

        Self { stack: *stack_ref }
    }
}

impl NetworkStack for Esp32NetworkStack {
    type Error = NetworkError;

    fn is_link_up(&self) -> bool {
        self.stack.is_link_up()
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        self.stack.wait_config_up().await;
        Ok(())
    }

    fn is_config_up(&self) -> bool {
        self.stack.config_v4().is_some()
    }
}

#[embassy_executor::task(pool_size = 1)]
async fn net_task(mut runner: Runner<'static, &'static mut WifiDevice<'static>>) {
    runner.run().await
}
