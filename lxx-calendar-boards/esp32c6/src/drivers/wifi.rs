use lxx_calendar_common::WifiController;
use lxx_calendar_common::*;

pub struct Esp32Wifi {
    connected: bool,
}

impl Esp32Wifi {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

impl Default for Esp32Wifi {
    fn default() -> Self {
        Self::new()
    }
}

impl WifiController for Esp32Wifi {
    type Error = core::convert::Infallible;

    async fn connect_sta(&mut self, ssid: &str, _password: &str) -> Result<(), Self::Error> {
        info!("ESP32 WiFi connecting to SSID: {}", ssid);
        // TODO: 使用 esp-radio 实现真正的 WiFi 连接
        // 需要使用 esp_radio::wifi 模块来连接 WiFi
        // 示例代码：
        // let config = ClientConfiguration { ... };
        // esp_radio::wifi::sta_connect(config).await?;
        self.connected = true;
        info!("ESP32 WiFi connected (stub)");
        todo!()
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        info!("ESP32 WiFi disconnecting");
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn get_rssi(&self) -> Result<i32, Self::Error> {
        Ok(-50)
    }
}
