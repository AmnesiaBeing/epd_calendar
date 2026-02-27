use core::convert::Infallible;
use esp_hal::peripherals::Peripherals;
use esp_hal::peripherals::BT;
use esp_radio::ble::controller::BleConnector;
use lxx_calendar_common::traits::ble::BLEDriver;
use lxx_calendar_common::*;
use bleps::{
    Ble,
    HciConnector,
    ad_structure::{
        AdStructure,
        BR_EDR_NOT_SUPPORTED,
        LE_GENERAL_DISCOVERABLE,
        create_advertising_data,
    },
    att::Uuid,
};

const DEVICE_NAME: &str = "LXX-Calendar";
const SERVICE_UUID: u16 = 0xFFF0;

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
        }
    }

    pub fn set_configured(&mut self, configured: bool) {
        self.configured = configured;
    }

    pub fn set_advertising(&mut self, advertising: bool) {
        self.advertising = advertising;
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

    let now = || esp_hal::time::Instant::now().duration_since_epoch().as_millis();
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
