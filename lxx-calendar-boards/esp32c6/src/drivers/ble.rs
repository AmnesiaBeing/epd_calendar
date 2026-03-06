use alloc::boxed::Box;
use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use esp_hal::peripherals::BT;
use esp_hal::peripherals::Peripherals;
use esp_radio::ble::controller::BleConnector;
use lxx_calendar_common::traits::ble::BLEDriver;
use lxx_calendar_common::*;
use trouble_host::prelude::*;

const DEVICE_NAME: &str = "LXX-Calendar";
const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 4;

static CONNECTED_FLAG: AtomicBool = AtomicBool::new(false);
static ADVERTISING_FLAG: AtomicBool = AtomicBool::new(false);
static BLE_STATE: AtomicU8 = AtomicU8::new(BLEState::Uninitialized as u8);

static OTA_TOTAL_SIZE: AtomicU32 = AtomicU32::new(0);
static OTA_RECEIVED: AtomicU32 = AtomicU32::new(0);
static OTA_DATA_OFFSET: AtomicU32 = AtomicU32::new(0);
static OTA_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BLEState {
    Uninitialized = 0,
    Initialized = 1,
    Advertising = 2,
    Connected = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BLEError {
    NotInitialized,
    AlreadyAdvertising,
    NotAdvertising,
    ConnectionFailed,
    DisconnectionFailed,
    GATTError,
    HostError,
    OTAError,
}

impl From<Infallible> for BLEError {
    fn from(_: Infallible) -> Self {
        BLEError::NotInitialized
    }
}

#[gatt_server]
struct CalendarServer {
    config_service: ConfigService,
    ota_service: OTAService,
}

#[gatt_service(uuid = "fff0")]
struct ConfigService {
    #[characteristic(uuid = "fff1", write, value = [0u8; 64])]
    network_config: [u8; 64],

    #[characteristic(uuid = "fff2", write, value = [0u8; 80])]
    time_config: [u8; 80],

    #[characteristic(uuid = "fff3", write, read, value = [0u8; 8])]
    display_config: [u8; 8],

    #[characteristic(uuid = "fff4", write, read, value = [0u8; 8])]
    power_config: [u8; 8],

    #[characteristic(uuid = "fff5", read, notify, value = 0u8)]
    status: u8,
}

#[gatt_service(uuid = "1819")]
struct OTAService {
    #[characteristic(uuid = "2a19", write, value = 0u8)]
    ota_control: u8,

    #[characteristic(uuid = "2a1a", write, value = [0u8; 20])]
    ota_data: [u8; 20],

    #[characteristic(uuid = "2a1b", read, notify, value = 0u8)]
    ota_status: u8,

    #[characteristic(uuid = "2a1c", read, notify, value = 0u32)]
    ota_progress: u32,
}

static DATA_CHANNEL: Channel<CriticalSectionRawMutex, heapless::Vec<u8, 256>, 4> = Channel::new();
static CONNECTED_CALLBACK: Mutex<CriticalSectionRawMutex, Option<Box<dyn Fn() + Send + 'static>>> = Mutex::new(None);
static DISCONNECTED_CALLBACK: Mutex<CriticalSectionRawMutex, Option<Box<dyn Fn() + Send + 'static>>> = Mutex::new(None);

pub struct Esp32BLE {
    initialized: bool,
}

impl Esp32BLE {
    pub fn new(spawner: embassy_executor::Spawner, peripherals: Peripherals) -> Self {
        let bt = peripherals.BT;
        spawner.spawn(ble_task(bt)).ok();
        Self { initialized: true }
    }

    pub fn is_connected_flag(&self) -> bool {
        CONNECTED_FLAG.load(Ordering::SeqCst)
    }
}

impl BLEDriver for Esp32BLE {
    type Error = BLEError;

    fn is_connected(&self) -> Result<bool, Self::Error> {
        Ok(CONNECTED_FLAG.load(Ordering::SeqCst))
    }

    fn is_advertising(&self) -> Result<bool, Self::Error> {
        Ok(ADVERTISING_FLAG.load(Ordering::SeqCst))
    }

    fn is_configured(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn start_advertising(&mut self) -> Result<(), Self::Error> {
        ADVERTISING_FLAG.store(true, Ordering::SeqCst);
        BLE_STATE.store(BLEState::Advertising as u8, Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Self::Error> {
        ADVERTISING_FLAG.store(false, Ordering::SeqCst);
        CONNECTED_FLAG.store(false, Ordering::SeqCst);
        BLE_STATE.store(BLEState::Initialized as u8, Ordering::SeqCst);
        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;
        BLE_STATE.store(BLEState::Initialized as u8, Ordering::SeqCst);
        Ok(())
    }

    async fn set_connected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        let mut guard = CONNECTED_CALLBACK.lock().await;
        *guard = Some(callback);
    }

    async fn set_disconnected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        let mut guard = DISCONNECTED_CALLBACK.lock().await;
        *guard = Some(callback);
    }

    async fn set_data_callback(&mut self, callback: Box<dyn Fn(&[u8]) + Send + 'static>) {
        loop {
            let data = DATA_CHANNEL.receive().await;
            callback(&data);
        }
    }

    async fn notify(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        info!("BLE notify: {} bytes", data.len());
        Ok(())
    }
}

#[embassy_executor::task]
async fn ble_task(bt: BT<'static>) {
    let radio = match esp_radio::init() {
        Ok(r) => r,
        Err(_) => {
            error!("Failed to init radio");
            return;
        }
    };

    let connector = match BleConnector::new(&radio, bt, Default::default()) {
        Ok(c) => c,
        Err(_) => {
            error!("Failed to create BLE connector");
            return;
        }
    };

    let controller: ExternalController<_, 20> = ExternalController::new(connector);

    let address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("BLE address: {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> = 
        HostResources::new();

    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);

    let Host {
        mut peripheral,
        runner,
        ..
    } = stack.build();

    let server = match CalendarServer::new_with_config(GapConfig::Peripheral(
        PeripheralConfig {
            name: DEVICE_NAME,
            appearance: &appearance::UNKNOWN,
        },
    )) {
        Ok(s) => s,
        Err(_) => {
            error!("Failed to create GATT server");
            return;
        }
    };

    BLE_STATE.store(BLEState::Initialized as u8, Ordering::SeqCst);
    info!("BLE initialized");

    let _ = join(
        ble_runner_task(runner),
        async {
            loop {
                match advertise_and_connect(&mut peripheral, &server).await {
                    Ok(conn) => {
                        CONNECTED_FLAG.store(true, Ordering::SeqCst);
                        BLE_STATE.store(BLEState::Connected as u8, Ordering::SeqCst);
                        info!("BLE connected");

                        {
                            let guard = CONNECTED_CALLBACK.lock().await;
                            if let Some(ref cb) = *guard {
                                cb();
                            }
                        }

                        let _ = gatt_events_task(&server, &conn).await;

                        CONNECTED_FLAG.store(false, Ordering::SeqCst);
                        BLE_STATE.store(BLEState::Initialized as u8, Ordering::SeqCst);
                        info!("BLE disconnected");

                        {
                            let guard = DISCONNECTED_CALLBACK.lock().await;
                            if let Some(ref cb) = *guard {
                                cb();
                            }
                        }
                    }
                    Err(e) => {
                        error!("BLE connection error: {:?}", e);
                        embassy_time::Timer::after_secs(1).await;
                    }
                }
            }
        },
    )
    .await;
}

async fn ble_runner_task<C: Controller, P: trouble_host::prelude::PacketPool>(
    mut runner: trouble_host::prelude::Runner<'_, C, P>,
) {
    loop {
        if let Err(e) = runner.run().await {
            error!("BLE runner error: {:?}", e);
        }
    }
}

async fn advertise_and_connect<'values, 'server, C: Controller>(
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server CalendarServer<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, trouble_host::BleHostError<C::Error>> {
    let mut adv_data = [0u8; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0xf0, 0xff]]),
            AdStructure::CompleteLocalName(DEVICE_NAME.as_bytes()),
        ],
        &mut adv_data[..],
    )?;

    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv_data[..len],
                scan_data: &[],
            },
        )
        .await?;

    ADVERTISING_FLAG.store(true, Ordering::SeqCst);
    info!("BLE advertising started");

    let conn = advertiser.accept().await?;
    ADVERTISING_FLAG.store(false, Ordering::SeqCst);

    info!("BLE connection established");
    Ok(conn.with_attribute_server(server)?)
}

async fn gatt_events_task(
    server: &CalendarServer<'_>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
) -> Result<(), trouble_host::Error> {
    let network_config = server.config_service.network_config;
    let time_config = server.config_service.time_config;
    let display_config = server.config_service.display_config;
    let power_config = server.config_service.power_config;
    let ota_control = server.ota_service.ota_control;
    let ota_data = server.ota_service.ota_data;

    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("BLE disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Write(e) => {
                        let handle = e.handle();
                        let data = e.data();

                        if handle == network_config.handle {
                            info!("Network config write: {} bytes", data.len());
                            if let Ok(vec) = heapless::Vec::<u8, 256>::from_slice(data) {
                                let _ = DATA_CHANNEL.send(vec).await;
                            }
                        } else if handle == time_config.handle {
                            info!("Time config write: {} bytes", data.len());
                            if let Ok(vec) = heapless::Vec::<u8, 256>::from_slice(data) {
                                let _ = DATA_CHANNEL.send(vec).await;
                            }
                        } else if handle == display_config.handle {
                            info!("Display config write: {} bytes", data.len());
                            if let Ok(vec) = heapless::Vec::<u8, 256>::from_slice(data) {
                                let _ = DATA_CHANNEL.send(vec).await;
                            }
                        } else if handle == power_config.handle {
                            info!("Power config write: {} bytes", data.len());
                            if let Ok(vec) = heapless::Vec::<u8, 256>::from_slice(data) {
                                let _ = DATA_CHANNEL.send(vec).await;
                            }
                        } else if handle == ota_control.handle {
                            handle_ota_control(data);
                        } else if handle == ota_data.handle {
                            handle_ota_data(data);
                        }
                    }
                    GattEvent::Read(e) => {
                        info!("GATT read: handle {:?}", e.handle());
                    }
                    _ => {}
                }

                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => {
                        info!("[gatt] error sending response: {:?}", e);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn handle_ota_control(data: &[u8]) {
    if data.is_empty() {
        return;
    }

    match data[0] {
        0x01 => {
            if data.len() >= 5 {
                let total_size = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                OTA_TOTAL_SIZE.store(total_size, Ordering::SeqCst);
                OTA_RECEIVED.store(0, Ordering::SeqCst);
                OTA_DATA_OFFSET.store(0, Ordering::SeqCst);
                OTA_IN_PROGRESS.store(true, Ordering::SeqCst);
                info!("OTA start: {} bytes", total_size);
            }
        }
        0x02 => {
            info!("OTA abort");
            OTA_TOTAL_SIZE.store(0, Ordering::SeqCst);
            OTA_RECEIVED.store(0, Ordering::SeqCst);
            OTA_DATA_OFFSET.store(0, Ordering::SeqCst);
            OTA_IN_PROGRESS.store(false, Ordering::SeqCst);
        }
        0x03 => {
            info!("OTA complete");
            OTA_IN_PROGRESS.store(false, Ordering::SeqCst);
        }
        0x04 => {
            info!("OTA mark valid and reboot");
            OTA_IN_PROGRESS.store(false, Ordering::SeqCst);
        }
        _ => {
            info!("Unknown OTA control: {:02x}", data[0]);
        }
    }
}

fn handle_ota_data(data: &[u8]) {
    if !OTA_IN_PROGRESS.load(Ordering::SeqCst) {
        info!("OTA data received but no OTA in progress");
        return;
    }

    let offset = OTA_DATA_OFFSET.load(Ordering::SeqCst);
    let total = OTA_TOTAL_SIZE.load(Ordering::SeqCst);
    
    if total == 0 {
        return;
    }

    info!("OTA data: {} bytes at offset {}", data.len(), offset);

    let new_offset = offset + data.len() as u32;
    OTA_DATA_OFFSET.store(new_offset, Ordering::SeqCst);
    OTA_RECEIVED.store(new_offset, Ordering::SeqCst);

    if new_offset >= total {
        info!("OTA transfer complete: {} bytes", new_offset);
        OTA_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
}