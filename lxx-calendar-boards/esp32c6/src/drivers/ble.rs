use alloc::boxed::Box;
use bleps::{
    Ble, HciConnector,
    ad_structure::{
        AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE, create_advertising_data,
    },
    att::Uuid,
};
use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::peripherals::BT;
use esp_hal::peripherals::Peripherals;
use esp_radio::ble::controller::BleConnector;
use lxx_calendar_common::traits::ble::BLEDriver;
use lxx_calendar_common::*;

const DEVICE_NAME: &str = "LXX-Calendar";
const SERVICE_UUID: u16 = 0xFFF0;

/// 全局静态回调函数存储
static CONNECTED_CALLBACK: Mutex<CriticalSectionRawMutex, Option<Box<dyn Fn() + Send + 'static>>> = Mutex::new(None);
static DISCONNECTED_CALLBACK: Mutex<CriticalSectionRawMutex, Option<Box<dyn Fn() + Send + 'static>>> = Mutex::new(None);
static DATA_CALLBACK: Mutex<CriticalSectionRawMutex, Option<Box<dyn Fn(&[u8]) + Send + 'static>>> = Mutex::new(None);
static CONNECTED_FLAG: AtomicBool = AtomicBool::new(false);

pub struct Esp32BLE {
    advertising: bool,
    connected: bool,
    configured: bool,
    initialized: bool,
}

impl Esp32BLE {
    pub fn new(spawner: embassy_executor::Spawner, peripherals: Peripherals) -> Self {
        let bt = peripherals.BT;

        spawner.spawn(ble_task(bt)).ok();

        Self {
            advertising: false,
            connected: false,
            configured: false,
            initialized: true,
        }
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
        if connected {
            self.advertising = false;
            CONNECTED_FLAG.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = CONNECTED_CALLBACK.lock() {
                if let Some(ref cb) = *guard {
                    cb();
                }
            }
        } else {
            CONNECTED_FLAG.store(false, Ordering::SeqCst);
            if let Ok(mut guard) = DISCONNECTED_CALLBACK.lock() {
                if let Some(ref cb) = *guard {
                    cb();
                }
            }
        }
    }

    pub fn set_configured(&mut self, configured: bool) {
        self.configured = configured;
    }

    pub fn set_advertising(&mut self, advertising: bool) {
        self.advertising = advertising;
    }

    pub fn is_connected_flag(&self) -> bool {
        CONNECTED_FLAG.load(Ordering::SeqCst)
    }

    pub fn get_data_callback(&self) -> &'static Mutex<CriticalSectionRawMutex, Option<Box<dyn Fn(&[u8]) + Send + 'static>>> {
        &DATA_CALLBACK
    }
}

impl BLEDriver for Esp32BLE {
    type Error = BLEError;

    fn is_connected(&self) -> Result<bool, Self::Error> {
        if !self.initialized {
            return Err(BLEError::NotInitialized);
        }
        Ok(self.connected)
    }

    fn is_advertising(&self) -> Result<bool, Self::Error> {
        if !self.initialized {
            return Err(BLEError::NotInitialized);
        }
        Ok(self.advertising)
    }

    fn is_configured(&self) -> Result<bool, Self::Error> {
        if !self.initialized {
            return Err(BLEError::NotInitialized);
        }
        Ok(self.configured)
    }

    fn start_advertising(&mut self) -> Result<(), Self::Error> {
        if !self.initialized {
            return Err(BLEError::NotInitialized);
        }
        self.advertising = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        if !self.initialized {
            return Err(BLEError::NotInitialized);
        }
        self.advertising = false;
        self.connected = false;
        Ok(())
    }

    fn initialize(&mut self) -> Result<(), Self::Error> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;
        Ok(())
    }

    fn set_connected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        if let Ok(mut guard) = CONNECTED_CALLBACK.lock() {
            *guard = Some(callback);
        }
    }

    async fn set_disconnected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        if let Ok(mut guard) = DISCONNECTED_CALLBACK.lock().await {
            *guard = Some(callback);
        }
    }

    async fn set_data_callback(&mut self, callback: Box<dyn Fn(&[u8]) + Send + 'static>) {
        if let Ok(mut guard) = DATA_CALLBACK.lock().await {
            *guard = Some(callback);
        }
    }

    fn notify(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        if !self.initialized {
            return Err(BLEError::NotInitialized);
        }
        info!("BLE notify: {} bytes", data.len());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BLEError {
    NotInitialized,
    AlreadyAdvertising,
    NotAdvertising,
    ConnectionFailed,
    DisconnectionFailed,
    GATTError,
}

impl From<Infallible> for BLEError {
    fn from(_: Infallible) -> Self {
        BLEError::NotInitialized
    }
}

#[embassy_executor::task]
async fn ble_task(bt: BT<'static>) {
    let radio = esp_radio::init().expect("Failed to init radio");
    let connector = match BleConnector::new(&radio, bt, Default::default()) {
        Ok(c) => c,
        Err(e) => {
            info!("Failed to create BLE connector: {:?}", e);
            return;
        }
    };

    let now = || {
        esp_hal::time::Instant::now()
            .duration_since_epoch()
            .as_millis()
    };
    let hci = HciConnector::new(connector, now);
    let mut ble = Ble::new(&hci);

    info!("BLE initializing...");
    if let Err(e) = ble.init() {
        info!("BLE init error: {:?}", e);
        return;
    }

    if let Err(e) = ble.cmd_set_le_advertising_parameters() {
        info!("BLE advertising params error: {:?}", e);
        return;
    }

    let advertising_data = match create_advertising_data(&[
        AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
        AdStructure::ServiceUuids16(&[Uuid::Uuid16(SERVICE_UUID)]),
        AdStructure::CompleteLocalName(DEVICE_NAME),
    ]) {
        Ok(data) => data,
        Err(e) => {
            info!("BLE advertising data create error: {:?}", e);
            return;
        }
    };

    if let Err(e) = ble.cmd_set_le_advertising_data(advertising_data) {
        info!("BLE advertising data error: {:?}", e);
        return;
    }

    if let Err(e) = ble.cmd_set_le_advertise_enable(true) {
        info!("BLE advertise enable error: {:?}", e);
        return;
    }

    info!("BLE started advertising");

    loop {
        embassy_time::Timer::after_millis(100).await;
    }
}