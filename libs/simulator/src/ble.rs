use lxx_calendar_common::traits::ble::BLEDriver;
use lxx_calendar_common::types::ConfigChange;
use lxx_calendar_common::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct SimulatedBLE {
    connected: bool,
    advertising: bool,
    configured: bool,
    connected_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
    disconnected_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
    data_callback: Arc<Mutex<Option<Box<dyn Fn(&[u8]) + Send + 'static>>>>,
    pub wakeup_flag: Arc<AtomicBool>,
}

impl SimulatedBLE {
    pub fn new() -> Self {
        Self {
            connected: false,
            advertising: false,
            configured: false,
            connected_callback: Arc::new(Mutex::new(None)),
            disconnected_callback: Arc::new(Mutex::new(None)),
            data_callback: Arc::new(Mutex::new(None)),
            wakeup_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_wakeup_flag(&self, flag: Arc<AtomicBool>) {
        // This replaces the internal flag with the external one
    }

    pub fn get_wakeup_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.wakeup_flag)
    }

    pub fn set_external_wakeup_flag(&mut self, flag: Arc<AtomicBool>) {
        self.wakeup_flag = flag;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn is_advertising(&self) -> bool {
        self.advertising
    }

    pub fn is_configured(&self) -> bool {
        self.configured
    }

    pub fn simulate_connect(&mut self) {
        self.connected = true;
        self.advertising = false;
        self.wakeup_flag.store(true, Ordering::SeqCst);
        info!("Simulated BLE connected");
        if let Ok(guard) = self.connected_callback.lock() {
            if let Some(ref cb) = *guard {
                cb();
            }
        }
    }

    pub fn simulate_disconnect(&mut self) {
        self.connected = false;
        info!("Simulated BLE disconnected");
        if let Ok(guard) = self.disconnected_callback.lock() {
            if let Some(ref cb) = *guard {
                cb();
            }
        }
    }

    pub fn simulate_config(&mut self, data: &[u8]) -> ConfigChange {
        self.configured = true;

        // 设置 wakeup flag 唤醒系统
        self.wakeup_flag.store(true, Ordering::SeqCst);

        let change = if data.len() < 10 {
            ConfigChange::TimeConfig
        } else if data.len() < 50 {
            ConfigChange::NetworkConfig
        } else if data.len() < 100 {
            ConfigChange::DisplayConfig
        } else if data.len() < 150 {
            ConfigChange::PowerConfig
        } else {
            ConfigChange::LogConfig
        };

        info!("Simulated BLE config applied: {:?}", change);

        if let Ok(guard) = self.data_callback.lock() {
            if let Some(ref cb) = *guard {
                cb(data);
            }
        }

        change
    }

    pub fn simulate_advertising(&mut self) {
        self.advertising = true;
        info!("Simulated BLE advertising");
    }
}

impl Default for SimulatedBLE {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SimulatedBLE {
    fn clone(&self) -> Self {
        Self {
            connected: self.connected,
            advertising: self.advertising,
            configured: self.configured,
            connected_callback: Arc::clone(&self.connected_callback),
            disconnected_callback: Arc::clone(&self.disconnected_callback),
            data_callback: Arc::clone(&self.data_callback),
            wakeup_flag: Arc::clone(&self.wakeup_flag),
        }
    }
}

impl BLEDriver for SimulatedBLE {
    type Error = core::convert::Infallible;

    fn is_connected(&self) -> Result<bool, Self::Error> {
        Ok(self.connected)
    }

    fn is_advertising(&self) -> Result<bool, Self::Error> {
        Ok(self.advertising)
    }

    fn is_configured(&self) -> Result<bool, Self::Error> {
        Ok(self.configured)
    }

    async fn start_advertising(&mut self) -> Result<(), Self::Error> {
        self.advertising = true;
        info!("Simulated BLE start advertising");
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Self::Error> {
        self.advertising = false;
        self.connected = false;
        info!("Simulated BLE stop");
        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        info!("Simulated BLE initialized");
        Ok(())
    }

    async fn set_connected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        if let Ok(mut guard) = self.connected_callback.lock() {
            *guard = Some(callback);
        }
    }

    async fn set_disconnected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        if let Ok(mut guard) = self.disconnected_callback.lock() {
            *guard = Some(callback);
        }
    }

    async fn set_data_callback(&mut self, callback: Box<dyn Fn(&[u8]) + Send + 'static>) {
        if let Ok(mut guard) = self.data_callback.lock() {
            *guard = Some(callback);
        }
    }

    async fn notify(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        info!("Simulated BLE notify: {} bytes", data.len());
        Ok(())
    }
}
