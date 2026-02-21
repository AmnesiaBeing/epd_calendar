use lxx_calendar_common::WifiController;
use lxx_calendar_common::*;

pub struct LinuxWifi {
    connected: bool,
    interface: Option<String>,
}

impl LinuxWifi {
    pub fn new() -> Self {
        Self {
            connected: false,
            interface: None,
        }
    }
}

impl Default for LinuxWifi {
    fn default() -> Self {
        Self::new()
    }
}

impl WifiController for LinuxWifi {
    type Error = core::convert::Infallible;

    async fn connect_sta(&mut self, ssid: &str, password: &str) -> Result<(), Self::Error> {
        use wifi_rs::WiFi;
        use wifi_rs::prelude::*;

        info!("Linux WiFi connecting to SSID: {}", ssid);

        let mut wifi = WiFi::new(None);

        match wifi.connect(ssid, password) {
            Ok(true) => {
                info!("Linux WiFi connected successfully");
                self.connected = true;
                self.interface = Some("wlan0".to_string());
            }
            Ok(false) => {
                warn!("Linux WiFi connection failed - invalid password");
                self.connected = false;
            }
            Err(e) => {
                error!("Linux WiFi connection error: {:?}", e);
                self.connected = false;
            }
        }

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        use wifi_rs::WiFi;
        use wifi_rs::prelude::*;

        info!("Linux WiFi disconnecting");

        let mut wifi = WiFi::new(self.interface.as_ref().map(|s| Config {
            interface: Some(s.as_str()),
        }));

        wifi.disconnect().ok();

        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
