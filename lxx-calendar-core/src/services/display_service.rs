use embassy_time::Duration;
use lxx_calendar_common::*;

pub struct DisplayService {
    initialized: bool,
    state: RefreshState,
    current_layout: DisplayLayout,
    last_refresh_time: Option<u64>,
    current_display_data: Option<DisplayData>,
    refresh_interval_seconds: u16,
    low_power_mode: bool,
}

impl DisplayService {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: RefreshState::Idle,
            current_layout: DisplayLayout::Default,
            last_refresh_time: None,
            current_display_data: None,
            refresh_interval_seconds: 60,
            low_power_mode: false,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing display service");
        self.state = RefreshState::Idle;
        self.initialized = true;
        Ok(())
    }

    pub async fn update_display(&mut self, data: DisplayData) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        info!("Updating display data");
        self.current_display_data = Some(data);

        if self.state == RefreshState::Idle {
            self.refresh().await?;
        }

        Ok(())
    }

    pub async fn refresh(&mut self) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }

        if self.state != RefreshState::Idle {
            info!("Display busy, skipping refresh");
            return Ok(());
        }

        info!("Refreshing display");

        let data = match self.current_display_data.clone() {
            Some(d) => d,
            None => {
                info!("No display data to refresh");
                return Ok(());
            }
        };

        if self.state == RefreshState::Idle {
            self.state = RefreshState::SendingData;

            match self.render_to_framebuffer(&data).await {
                Ok(_) => {
                    self.state = RefreshState::Refreshing;

                    embassy_time::Timer::after(Duration::from_secs(10)).await;

                    self.state = RefreshState::Idle;
                    self.last_refresh_time = Some(embassy_time::Instant::now().elapsed().as_secs());
                    info!("Display refreshed successfully");
                }
                Err(e) => {
                    self.state = RefreshState::Error(RefreshError::CommunicationError);
                    error!("Display refresh failed: {:?}", e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    async fn render_to_framebuffer(&mut self, data: &DisplayData) -> SystemResult<()> {
        info!(
            "Rendering: time={}-{:02}-{:02} {:02}:{:02}, low_battery={}",
            data.solar_time.get_year(),
            data.solar_time.get_month(),
            data.solar_time.get_day(),
            data.solar_time.get_hour(),
            data.solar_time.get_minute(),
            data.low_battery
        );
        Ok(())
    }

    pub async fn set_refresh_interval(&mut self, seconds: u16) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        self.refresh_interval_seconds = seconds;
        Ok(())
    }

    pub async fn enter_low_power_mode(&mut self) -> SystemResult<()> {
        self.low_power_mode = true;
        info!("Display entered low power mode");
        Ok(())
    }

    pub async fn exit_low_power_mode(&mut self) -> SystemResult<()> {
        self.low_power_mode = false;
        info!("Display exited low power mode");
        Ok(())
    }

    pub async fn show_qrcode(&mut self, ssid: &str) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        info!("Showing QR code for SSID: {}", ssid);
        self.current_layout = DisplayLayout::LargeTime;
        Ok(())
    }
}
