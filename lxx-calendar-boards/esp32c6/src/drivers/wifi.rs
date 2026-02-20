use alloc::borrow::ToOwned;
use esp_hal::peripherals::Peripherals;
use esp_radio::wifi::ClientConfig;
use lxx_calendar_common::WifiController;
use lxx_calendar_common::*;

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

pub struct Esp32Wifi {
    controller: esp_radio::wifi::WifiController<'static>,
}

impl Esp32Wifi {
    pub fn new(
        peripherals: &Peripherals,
    ) -> (Self, &'static mut esp_radio::wifi::WifiDevice<'static>) {
        let esp_radio_controller =
            mk_static!(esp_radio::Controller<'static>, esp_radio::init().unwrap());

        let (controller, interfaces) = esp_radio::wifi::new(
            esp_radio_controller,
            unsafe { peripherals.WIFI.clone_unchecked() },
            Default::default(),
        )
        .unwrap();

        let interfaces = mk_static!(esp_radio::wifi::Interfaces<'static>, interfaces);

        (Self { controller }, &mut interfaces.sta)
    }
}

impl WifiController for Esp32Wifi {
    type Error = WifiError;

    async fn connect_sta(&mut self, ssid: &str, password: &str) -> Result<(), Self::Error> {
        info!("ESP32 WiFi async connecting to SSID: {}", ssid);
        let config = ClientConfig::default()
            .with_ssid(ssid.to_owned())
            .with_password(password.to_owned());
        self.controller
            .set_config(&esp_radio::wifi::ModeConfig::Client(config))
            .unwrap();
        self.controller.connect_async().await?;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        self.controller.disconnect_async().await?;
        info!("WiFi disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.controller.is_connected().is_ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiError {
    NotInitialized,
    ConfigFailed,
    ConnectionFailed,
    DisconnectionFailed,
}

impl core::fmt::Display for WifiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WifiError::NotInitialized => write!(f, "WiFi controller not initialized"),
            WifiError::ConfigFailed => write!(f, "WiFi configuration failed"),
            WifiError::ConnectionFailed => write!(f, "WiFi connection failed"),
            WifiError::DisconnectionFailed => write!(f, "WiFi disconnection failed"),
        }
    }
}

impl From<esp_radio::wifi::WifiError> for WifiError {
    fn from(error: esp_radio::wifi::WifiError) -> Self {
        match error {
            esp_radio::wifi::WifiError::NotInitialized => WifiError::NotInitialized,
            esp_radio::wifi::WifiError::InvalidArguments => WifiError::ConfigFailed,
            esp_radio::wifi::WifiError::Disconnected => WifiError::DisconnectionFailed,
            _ => WifiError::ConnectionFailed,
        }
    }
}
