use embassy_executor::Spawner;
use embedded_hal_mock::eh1::{
    delay::NoopDelay,
    digital::{Mock as PinMock, State, Transaction as PinTransaction},
    spi::{Mock as SpiMock, Transaction as SpiTransaction},
};
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;
use simulated_rtc::SimulatedRtc;
use simulated_wdt::SimulatedWdt;

pub struct SimulatorBuzzer;

impl BuzzerDriver for SimulatorBuzzer {
    type Error = core::convert::Infallible;

    fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), Self::Error> {
        // Simulator: 只记录日志
        info!(
            "[Simulator Buzzer] Playing {}Hz for {}ms",
            frequency, duration_ms
        );

        // 如果需要实际播放声音，可以使用 rodio 等库
        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));

        Ok(())
    }
}

pub struct SimulatorNetwork;

impl SimulatorNetwork {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimulatorNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStack for SimulatorNetwork {
    type Error = core::convert::Infallible;

    fn is_link_up(&self) -> bool {
        false
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        info!("[Simulator Network] Waiting for config (stub)");
        Ok(())
    }

    fn is_config_up(&self) -> bool {
        false
    }
}

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

    type AudioDevice = SimulatorBuzzer;

    type RtcDevice = SimulatedRtc;

    type WifiDevice = NoWifi;

    type NetworkStack = SimulatorNetwork;

    async fn init(spawner: Spawner) -> PlatformContext<Self> {
        let wdt = SimulatedWdt::new(5000);
        simulated_wdt::start_watchdog(&spawner, 5000);

        let spi = create_spi_mock();
        let _busy = create_input_pin_mock();
        let _dc = create_output_pin_mock();
        let _rst = create_output_pin_mock();
        let _delay = NoopDelay::new();

        let audio = SimulatorBuzzer;

        let mut rtc = SimulatedRtc::new();
        rtc.initialize().await.ok();

        let wifi = NoWifi::new();
        let network = SimulatorNetwork::new();

        PlatformContext {
            sys_watch_dog: wdt,
            epd: spi,
            audio,
            rtc,
            wifi,
            network,
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
