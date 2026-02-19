use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use epd_yrd0750ryf665f60::{prelude::WaveshareDisplay as _, yrd0750ryf665f60::Epd7in5};
use esp_hal::peripherals::Peripherals;
use static_cell::StaticCell;

use crate::Platform;

impl Platform {
    pub(crate) async fn init_epd(
        peripherals: &Peripherals,
    ) -> <Platform as lxx_calendar_common::PlatformTrait>::EpdDevice {
        static SPI_BUS_MUTEX: StaticCell<
            embassy_sync::mutex::Mutex<
                CriticalSectionRawMutex,
                esp_hal::spi::master::Spi<'static, esp_hal::Async>,
            >,
        > = StaticCell::new();
        static EPD_DEVICE: StaticCell<
            embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
                CriticalSectionRawMutex,
                esp_hal::spi::master::Spi<'static, esp_hal::Async>,
                esp_hal::gpio::Output<'static>,
            >,
        > = StaticCell::new();

        let sck: esp_hal::peripherals::GPIO22<'static> =
            unsafe { peripherals.GPIO22.clone_unchecked() };
        let sda: esp_hal::peripherals::GPIO23<'static> =
            unsafe { peripherals.GPIO23.clone_unchecked() };
        let cs: esp_hal::gpio::Output<'static> = esp_hal::gpio::Output::new(
            unsafe { peripherals.GPIO21.clone_unchecked() },
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );

        let busy: esp_hal::gpio::Input<'static> = esp_hal::gpio::Input::new(
            unsafe { peripherals.GPIO18.clone_unchecked() },
            esp_hal::gpio::InputConfig::default(),
        );
        let dc: esp_hal::gpio::Output<'static> = esp_hal::gpio::Output::new(
            unsafe { peripherals.GPIO20.clone_unchecked() },
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );
        let rst: esp_hal::gpio::Output<'static> = esp_hal::gpio::Output::new(
            unsafe { peripherals.GPIO19.clone_unchecked() },
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default(),
        );

        let spi2: esp_hal::peripherals::SPI2<'static> =
            unsafe { peripherals.SPI2.clone_unchecked() };

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

        let spi_bus_mutex = embassy_sync::mutex::Mutex::new(spi_bus);
        let spi_bus_mutex_static: &'static _ = SPI_BUS_MUTEX.init(spi_bus_mutex);

        let epd_device =
            embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(spi_bus_mutex_static, cs);
        let epd_device_static: &'static mut _ = EPD_DEVICE.init(epd_device);

        let mut delay = embassy_time::Delay;

        let epd = Epd7in5::new(epd_device_static, busy, dc, rst, &mut delay)
            .await
            .unwrap();

        epd
    }
}
