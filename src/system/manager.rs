use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};

use crate::platform::Platform;
use crate::system::MessageBus;

pub struct SystemManager<P: Platform> {
    platform: P,
    message_bus: MessageBus,
}

impl<P: Platform> SystemManager<P> {
    pub async fn new(
        mut platform: P,
        message_bus: MessageBus,
        spawner: &Spawner,
    ) -> core::result::Result<Self, crate::common::error::AppError> {
        let display_driver = platform.create_display_driver().await?;
        let mut network_driver = platform.create_network_driver(spawner).await?;
        network_driver.initialize(spawner).await?;
        let buzzer_driver = platform.create_buzzer_driver()?;
        let time_driver = platform.create_time_driver()?;
        let storage_driver = platform.create_storage_driver()?;
        let power_driver = platform.create_power_driver()?;
        let sensor_driver = platform.create_sensor_driver()?;
        let led_driver = platform.create_led_driver(spawner)?;
        let button_driver = platform.create_button_driver()?;

        Ok(Self {
            platform,
            message_bus,
        })
    }

    pub async fn run(&self) {
        let mut last_system_check = Instant::now();
        const SYSTEM_CHECK_INTERVAL: Duration = Duration::from_secs(60);

        log::info!("System manager running");

        loop {
            if last_system_check.elapsed() > SYSTEM_CHECK_INTERVAL {
                self.log_system_health().await;
                last_system_check = Instant::now();
            }

            Timer::after(Duration::from_secs(30)).await;
        }
    }

    async fn log_system_health(&self) {
        log::debug!("System health check");
        log::debug!("System health check completed");
    }

    pub fn message_bus(&self) -> &MessageBus {
        &self.message_bus
    }
}
