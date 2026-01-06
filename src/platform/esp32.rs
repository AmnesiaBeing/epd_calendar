use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_hal::peripherals::Peripherals;
use esp_hal::timer::timg::TimerGroup;

use crate::common::error::Result;
use crate::platform::common::Platform;

esp_bootloader_esp_idf::esp_app_desc!();

use panic_rtt_target as _;

pub struct Esp32Platform {
    peripherals: Peripherals,
}

impl Platform for Esp32Platform {
    type Peripherals = Peripherals;

    fn init() -> Result<Self> {
        let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

        esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);

        Ok(Self { peripherals })
    }

    fn peripherals(&self) -> &Self::Peripherals {
        &self.peripherals
    }

    fn peripherals_mut(&mut self) -> &mut Self::Peripherals {
        &mut self.peripherals
    }

    fn init_logging(&self) {
        rtt_target::rtt_init_log!();
        log::info!("Initializing logger for ESP32");
    }

    fn init_rtos(&mut self) {
        let timg0 = TimerGroup::new(unsafe { self.peripherals.TIMG0.clone_unchecked() });

        let sw_interrupt = esp_hal::interrupt::software::SoftwareInterruptControl::new(unsafe {
            self.peripherals.SW_INTERRUPT.clone_unchecked()
        });

        esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    }
}
