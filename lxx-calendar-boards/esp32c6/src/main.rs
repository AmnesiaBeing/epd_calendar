#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
#![no_std]
#![no_main]

extern crate alloc;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp_hal::{interrupt::software::SoftwareInterruptControl, timer::timg::TimerGroup};
use esp_rtos::main as platform_main;
use lxx_calendar_common::*;
use lxx_calendar_core::main_task;

pub mod drivers;

esp_bootloader_esp_idf::esp_app_desc!();

use panic_rtt_target as _;

use static_cell::StaticCell;

use crate::drivers::{
    Esp32Battery, Esp32Button, Esp32Buzzer, Esp32LED, Esp32NetworkStack, Esp32Rtc, Esp32Watchdog,
    Esp32Wifi,
};

pub struct Platform;

impl PlatformTrait for Platform {
    type WatchdogDevice = Esp32Watchdog;

    type EpdDevice = epd_yrd0750ryf665f60::yrd0750ryf665f60::Epd7in5<
        embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
            'static,
            CriticalSectionRawMutex,
            esp_hal::spi::master::Spi<'static, esp_hal::Async>,
            esp_hal::gpio::Output<'static>,
        >,
        esp_hal::gpio::Input<'static>,
        esp_hal::gpio::Output<'static>,
        esp_hal::gpio::Output<'static>,
        embassy_time::Delay,
    >;

    type AudioDevice = Esp32Buzzer;

    type ButtonDevice = Esp32Button;

    type LEDDevice = Esp32LED;

    type RtcDevice = Esp32Rtc;

    type WifiDevice = Esp32Wifi;

    type NetworkStack = Esp32NetworkStack;

    type BatteryDevice = Esp32Battery;

    async fn init(spawner: embassy_executor::Spawner) -> SystemResult<PlatformContext<Self>> {
        let peripherals = esp_hal::init(
            esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max()),
        );

        let timg0 = TimerGroup::new(unsafe { peripherals.TIMG0.clone_unchecked() });
        let sw_int =
            SoftwareInterruptControl::new(unsafe { peripherals.SW_INTERRUPT.clone_unchecked() });
        esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

        let sys_watch_dog = Esp32Watchdog::new(&peripherals);
        let audio = Esp32Buzzer::new(&peripherals);
        let led = Esp32LED::new(&peripherals, spawner);
        let battery = Esp32Battery::new(&peripherals);
        let rtc = Esp32Rtc::new(&peripherals);
        let (wifi, wifi_interface) = Esp32Wifi::new(&peripherals);
        let network = Esp32NetworkStack::new(spawner, wifi_interface);
        let epd = Self::init_epd(&peripherals).await;
        let button = Esp32Button::new(&peripherals, spawner);

        Ok(PlatformContext {
            sys_watch_dog,
            epd,
            button,
            audio,
            led,
            rtc,
            wifi,
            network,
            battery,
        })
    }

    fn sys_reset() {
        esp_hal::system::software_reset()
    }

    fn init_logger() {
        rtt_target::rtt_init_defmt!();
    }

    fn init_heap() {
        esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 64 * 1024);
        esp_alloc::heap_allocator!(size: 64 * 1024);
    }
}

#[platform_main]
async fn main(spawner: embassy_executor::Spawner) {
    static EVENT_CHANNEL: StaticCell<LxxSystemEventChannel> = StaticCell::new();
    let event_channel = EVENT_CHANNEL.init(LxxSystemEventChannel::new());
    let event_sender = event_channel.sender();
    let event_receiver = event_channel.receiver();

    match Platform::init(spawner).await {
        Ok(platform_ctx) => {
            if let Err(e) =
                main_task::<Platform>(spawner, event_receiver, event_sender, platform_ctx).await
            {
                error!("Main task error: {:?}", e);
            }
        }
        Err(e) => {
            error!("Platform init error: {:?}", e);
        }
    }
}
