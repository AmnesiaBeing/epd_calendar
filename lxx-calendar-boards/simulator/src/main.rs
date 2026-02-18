use embassy_executor::Spawner;
use embedded_hal_mock::eh1::{
    delay::NoopDelay,
    digital::{Mock as PinMock, State, Transaction as PinTransaction},
    spi::{Mock as SpiMock, Transaction as SpiTransaction},
};
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use simulated_wdt::SimulatedWdt;

pub struct Platform;

type MockSpi = SpiMock<u8>;
type MockPin = PinMock;

fn create_spi_mock() -> MockSpi {
    let transactions = [
        SpiTransaction::transaction_start(),
        SpiTransaction::transaction_end(),
    ];
    let spi = MockSpi::new(&transactions);
    spi
}

fn create_input_pin_mock() -> MockPin {
    let transactions = [PinTransaction::get(State::High)];
    PinMock::new(&transactions)
}

fn create_output_pin_mock() -> MockPin {
    let transactions = [PinTransaction::set(State::High)];
    PinMock::new(&transactions)
}

impl PlatformTrait for Platform {
    type WatchdogDevice = SimulatedWdt;

    type EpdDevice = MockSpi;

    async fn init(spawner: Spawner) -> PlatformContext<Self> {
        let wdt = SimulatedWdt::new(5000);
        simulated_wdt::start_watchdog(&spawner, 5000);

        let spi = create_spi_mock();
        let _busy = create_input_pin_mock();
        let _dc = create_output_pin_mock();
        let _rst = create_output_pin_mock();
        let _delay = NoopDelay::new();

        PlatformContext {
            sys_watch_dog: wdt,
            epd: spi,
        }
    }

    fn sys_reset() {
        info!("Simulator platform reset");
    }

    fn sys_stop() {
        info!("Simulator platform stop");
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let platform_ctx = Platform::init(spawner).await;
    if let Err(e) = main_task::<Platform>(spawner, platform_ctx).await {
        error!("Main task error: {:?}", e);
    }
}
