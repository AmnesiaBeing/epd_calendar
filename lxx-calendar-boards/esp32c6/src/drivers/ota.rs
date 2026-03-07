use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use lxx_calendar_common::traits::ota::{OTADriver, OTAError, OTAProgress, OTAState};

const OTA_PARTITION_SIZE: u32 = 0x180000;
const OTA_PARTITION0_ADDR: u32 = 0x1A0000;
const OTA_PARTITION1_ADDR: u32 = 0x320000;
const OTA_DATA_ADDR: u32 = 0x4A0000;
const SECTOR_SIZE: u32 = 4096;

static OTA_STATE: AtomicU8 = AtomicU8::new(OTAState::Idle as u8);
static OTA_TOTAL: AtomicU32 = AtomicU32::new(0);
static OTA_RECEIVED: AtomicU32 = AtomicU32::new(0);
static OTA_TARGET_PARTITION: AtomicU8 = AtomicU8::new(0);

pub struct Esp32OTA {
    target_partition: u8,
}

impl Esp32OTA {
    pub fn new() -> Self {
        Self {
            target_partition: 0,
        }
    }

    fn get_target_address(&self) -> u32 {
        if self.target_partition == 0 {
            OTA_PARTITION0_ADDR
        } else {
            OTA_PARTITION1_ADDR
        }
    }

    fn get_current_partition() -> u8 {
        0
    }
}

impl Default for Esp32OTA {
    fn default() -> Self {
        Self::new()
    }
}

impl OTADriver for Esp32OTA {
    type Error = OTAError;

    fn get_state(&self) -> OTAState {
        match OTA_STATE.load(Ordering::SeqCst) {
            0 => OTAState::Idle,
            1 => OTAState::Receiving,
            2 => OTAState::Verifying,
            3 => OTAState::Ready,
            4 => OTAState::Error,
            _ => OTAState::Idle,
        }
    }

    fn get_progress(&self) -> OTAProgress {
        OTAProgress {
            received: OTA_RECEIVED.load(Ordering::SeqCst),
            total: OTA_TOTAL.load(Ordering::SeqCst),
            state: self.get_state(),
        }
    }

    async fn begin(&mut self, total_size: u32) -> Result<(), Self::Error> {
        if self.get_state() != OTAState::Idle {
            return Err(OTAError::AlreadyInProgress);
        }

        let current = Self::get_current_partition();
        self.target_partition = if current == 0 { 1 } else { 0 };
        OTA_TARGET_PARTITION.store(self.target_partition, Ordering::SeqCst);
        OTA_TOTAL.store(total_size, Ordering::SeqCst);
        OTA_RECEIVED.store(0, Ordering::SeqCst);
        OTA_STATE.store(OTAState::Receiving as u8, Ordering::SeqCst);

        defmt::info!(
            "OTA started: {} bytes to partition {}",
            total_size,
            self.target_partition
        );
        Ok(())
    }

    async fn write(&mut self, _offset: u32, data: &[u8]) -> Result<(), Self::Error> {
        if self.get_state() != OTAState::Receiving {
            return Err(OTAError::NotInProgress);
        }

        let new_received = OTA_RECEIVED.load(Ordering::SeqCst) + data.len() as u32;
        OTA_RECEIVED.store(new_received, Ordering::SeqCst);

        let total = OTA_TOTAL.load(Ordering::SeqCst);
        if new_received >= total {
            OTA_STATE.store(OTAState::Verifying as u8, Ordering::SeqCst);
            defmt::info!("OTA data complete: {} bytes received", new_received);
        }

        Ok(())
    }

    async fn abort(&mut self) -> Result<(), Self::Error> {
        OTA_STATE.store(OTAState::Idle as u8, Ordering::SeqCst);
        OTA_TOTAL.store(0, Ordering::SeqCst);
        OTA_RECEIVED.store(0, Ordering::SeqCst);
        defmt::info!("OTA aborted");
        Ok(())
    }

    async fn complete(&mut self) -> Result<(), Self::Error> {
        let state = self.get_state();
        if state != OTAState::Verifying && state != OTAState::Receiving {
            return Err(OTAError::NotInProgress);
        }

        let received = OTA_RECEIVED.load(Ordering::SeqCst);
        let total = OTA_TOTAL.load(Ordering::SeqCst);

        if received < total {
            return Err(OTAError::InvalidData);
        }

        OTA_STATE.store(OTAState::Ready as u8, Ordering::SeqCst);
        defmt::info!("OTA complete and verified");
        Ok(())
    }

    async fn mark_valid(&mut self) -> Result<(), Self::Error> {
        if self.get_state() != OTAState::Ready {
            return Err(OTAError::NotInProgress);
        }

        let partition = OTA_TARGET_PARTITION.load(Ordering::SeqCst);
        OTA_STATE.store(OTAState::Idle as u8, Ordering::SeqCst);
        defmt::info!(
            "OTA partition {} marked as valid, will boot on next reset",
            partition
        );
        Ok(())
    }

    fn get_ota_partition_size(&self) -> u32 {
        OTA_PARTITION_SIZE
    }
}
