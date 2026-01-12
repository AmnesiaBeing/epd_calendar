#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
#![no_std]

mod mutex;

use core::marker::PhantomData;

use epd_yrd0750ryf665f60::prelude::WaveshareDisplay as _;
use esp_hal::timer::timg::{TimerGroup, Wdt};
pub use esp_rtos::main as platform_main;
use lxx_calendar_common::*;
pub use mutex::*;

esp_bootloader_esp_idf::esp_app_desc!();

use panic_rtt_target as _;

pub struct Platform;

impl PlatformTrait for Platform {
    type StaticWatchDogControllerMutexType =
        LxxAsyncMutex<Wdt<esp_hal::peripherals::TIMG0<'static>>>;

    type StatiEpdControllerMutexType = LxxAsyncMutex<
        epd_yrd0750ryf665f60::yrd0750ryf665f60::Epd7in5<
            embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
                'static,
                embassy_sync::blocking_mutex::raw::NoopRawMutex,
                esp_hal::spi::master::Spi<'static, esp_hal::Async>,
                esp_hal::gpio::Output<'static>,
            >,
            esp_hal::gpio::Input<'static>,
            esp_hal::gpio::Output<'static>,
            esp_hal::gpio::Output<'static>,
            embassy_time::Delay,
        >,
    >;

    async fn init(_spawner: embassy_executor::Spawner) -> PlatformContext<Self> {
        let mut peripherals = esp_hal::init(
            esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max()),
        );
        esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 32768);

        let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        let sys_watch_dog = LxxAsyncMutex::new(timg0.wdt);

        let sck = peripherals.GPIO22;
        let sda = peripherals.GPIO23;
        let cs: esp_hal::gpio::Output<'_> = esp_hal::gpio::Output::new(
            peripherals.GPIO21,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );

        // 配置 EPD 控制引脚
        let busy =
            esp_hal::gpio::Input::new(peripherals.GPIO18, esp_hal::gpio::InputConfig::default());
        let dc = esp_hal::gpio::Output::new(
            peripherals.GPIO20,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );
        let rst = esp_hal::gpio::Output::new(
            peripherals.GPIO19,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );

        // 获取 SPI2 实例
        let spi2 = peripherals.SPI2;

        // 创建 SPI 总线
        let spi_bus = esp_hal::spi::master::Spi::new(
            spi2,
            esp_hal::spi::master::Config::default()
                .with_frequency(esp_hal::time::Rate::from_mhz(10))
                .with_mode(esp_hal::spi::Mode::_0),
        )
        .unwrap()
        .with_sck(sck)
        .with_sio0(sda)
        .into_async();

        let mut delay = embassy_time::Delay;
        let spi_bus_mutex = LxxAsyncMutex::new(spi_bus);
        let mut spi_device =
            embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(&spi_bus_mutex, cs);

        let epd = LxxAsyncMutex::new(
            epd_yrd0750ryf665f60::yrd0750ryf665f60::Epd7in5::new(
                &mut spi_device,
                busy,
                dc,
                rst,
                &mut delay,
            )
            .await
            .unwrap(),
        );

        PlatformContext { sys_watch_dog, epd }
    }

    fn sys_reset() {
        todo!()
    }

    fn sys_stop() {
        todo!()
    }
}
