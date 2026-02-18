use embassy_executor::Spawner;
use lxx_calendar_common::*;
use simulated_wdt::SimulatedWdt;

pub struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = SimulatedWdt;

    type EpdDevice = MockSpiDevice;

    async fn init(spawner: Spawner) -> PlatformContext<Self> {
        let wdt = SimulatedWdt::new(5000);
        wdt.start_watchdog_task(spawner);

        PlatformContext {
            sys_watch_dog: wdt,
            epd: MockSpiDevice::new(),
        }
    }

    fn sys_reset() {
        info!("Simulator platform reset");
    }

    fn sys_stop() {
        info!("Simulator platform stop");
    }
}
