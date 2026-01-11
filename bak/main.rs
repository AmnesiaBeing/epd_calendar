#![cfg_attr(feature = "esp32c6", no_std)]
#![cfg_attr(feature = "esp32c6", no_main)]
#![cfg_attr(
    feature = "esp32c6",
    deny(
        clippy::mem_forget,
        reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for duration of a data transfer."
    )
)]
#![cfg_attr(feature = "esp32c6", deny(clippy::large_stack_frames))]

extern crate alloc;

use embassy_executor::Spawner;

mod assets;
mod common;
mod kernel;
mod platform;

use crate::common::error::Result;
use crate::kernel::data::DataSourceRegistry;
use crate::kernel::data::generic_scheduler_task;
use crate::kernel::data::sources::config::ConfigDataSource;
use crate::kernel::data::sources::motto::MottoDataSource;
use crate::kernel::data::sources::time::TimeDataSource;
use crate::kernel::data::sources::weather::WeatherDataSource;
use crate::kernel::driver::button::DefaultButtonDriver;
use crate::kernel::driver::buzzer::DefaultBuzzerDriver;
use crate::kernel::driver::epd::DefaultDisplayDriver;
use crate::kernel::driver::epd::DisplayDriver;
use crate::kernel::driver::led::DefaultLedDriver;
use crate::kernel::driver::network::DefaultNetworkDriver;
use crate::kernel::driver::network::NetworkDriver;
use crate::kernel::driver::ntp_source::SntpService;
use crate::kernel::driver::power::DefaultPowerDriver;
use crate::kernel::driver::sensor::DefaultSensorDriver;
use crate::kernel::driver::storage::DefaultConfigStorageDriver;
use crate::kernel::driver::rtc::DefaultTimeDriver;
use crate::kernel::system::api::DefaultSystemApi;
use crate::platform::{DefaultPlatform, Platform};

#[cfg(any(feature = "simulator", feature = "tspi"))]
use embassy_executor::main as platform_main;

#[cfg(feature = "esp32c6")]
use esp_rtos::main as platform_main;

#[platform_main]
async fn main(spawner: Spawner) {
    let message_bus = MessageBus::new();

    let mut platform = DefaultPlatform::init().unwrap();

    platform.init_logging();

    platform.init_rtos();

    run_system(platform, message_bus, &spawner).await;
}

async fn run_system(
    mut platform: DefaultPlatform,
    message_bus: MessageBus,
    spawner: &Spawner,
) -> Result<()> {
    let display_driver = DefaultDisplayDriver::create(platform.peripherals_mut())?;
    let mut network_driver = DefaultNetworkDriver::create(platform.peripherals_mut())?;
    network_driver.new(spawner).await.unwrap();
    let buzzer_driver = platform.create_buzzer_driver().unwrap();
    let time_driver = platform.create_time_driver().unwrap();
    let storage_driver = platform.create_storage_driver().unwrap();
    let power_driver = platform.create_power_driver().unwrap();
    let sensor_driver = platform.create_sensor_driver().unwrap();
    let led_driver = platform.create_led_driver(spawner).unwrap();
    let button_driver = platform.create_button_driver().unwrap();

    let display_driver_static = common::GlobalMutex::new(display_driver);
    let network_driver_static = common::GlobalMutex::new(network_driver);
    let buzzer_driver_static = common::GlobalMutex::new(buzzer_driver);
    let time_driver_static = common::GlobalMutex::new(time_driver);

    SntpService::initialize(spawner, &network_driver_static, &time_driver_static);

    let system_api = DefaultSystemApi::new(
        power_driver,
        &network_driver_static,
        storage_driver,
        sensor_driver,
        led_driver,
        &buzzer_driver_static,
        &display_driver_static,
    );

    let data_source_registry = DataSourceRegistry::init();

    let config_source_mutex =
        common::GlobalMutex::new(ConfigDataSource::new(&system_api).await.unwrap());
    data_source_registry
        .lock()
        .await
        .register_source(&config_source_mutex, &system_api)
        .await
        .unwrap();

    let time_source_mutex =
        common::GlobalMutex::new(TimeDataSource::new(&time_driver_static).unwrap());
    data_source_registry
        .lock()
        .await
        .register_source(&time_source_mutex, &system_api)
        .await
        .unwrap();

    let weather_source_mutex =
        common::GlobalMutex::new(WeatherDataSource::new(&system_api).await.unwrap());
    data_source_registry
        .lock()
        .await
        .register_source(&weather_source_mutex, &system_api)
        .await
        .unwrap();

    let motto_source_mutex = common::GlobalMutex::new(MottoDataSource::new().unwrap());
    data_source_registry
        .lock()
        .await
        .register_source(&motto_source_mutex, &system_api)
        .await
        .unwrap();

    let system_api_static = common::GlobalMutex::new(system_api);
    let data_source_registry_static = data_source_registry;

    spawner
        .spawn(main_task(
            &display_driver_static,
            data_source_registry_static,
        ))
        .unwrap();

    spawner
        .spawn(generic_scheduler_task(
            data_source_registry_static,
            &system_api_static,
        ))
        .unwrap();

    embassy_time::Timer::after(embassy_time::Duration::from_secs(3600)).await;

    Ok(())
}
