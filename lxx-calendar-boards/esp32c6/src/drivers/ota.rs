use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_storage::nor_flash::{NorFlash as SyncNorFlash, ReadNorFlash as SyncReadNorFlash};
use embedded_storage_async::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};
use lxx_calendar_common::flash_layout::{OTA_0_OFFSET, OTA_0_SIZE, OTA_1_OFFSET, OTA_STATE_OFFSET};
use lxx_calendar_common::traits::ota::{OTADriver, OTAError, OTAProgress, OTAState};

const SECTOR_SIZE: u32 = 4096;

static OTA_STATE: AtomicU8 = AtomicU8::new(OTAState::Idle as u8);
static OTA_TOTAL: AtomicU32 = AtomicU32::new(0);
static OTA_RECEIVED: AtomicU32 = AtomicU32::new(0);
static OTA_TARGET_PARTITION: AtomicU8 = AtomicU8::new(0);

static FLASH_MUTEX: Mutex<CriticalSectionRawMutex, Option<EspFlashWrapper>> = Mutex::new(None);

struct EspFlashWrapper {
    inner: esp_storage::FlashStorage<'static>,
}

impl EspFlashWrapper {
    fn new(flash: esp_hal::peripherals::FLASH<'static>) -> Self {
        Self {
            inner: esp_storage::FlashStorage::new(flash),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Esp32FlashError {
    Flash(esp_storage::FlashStorageError),
    NotInitialized,
}

impl NorFlashError for Esp32FlashError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Esp32FlashError::Flash(e) => match e {
                esp_storage::FlashStorageError::NotAligned => NorFlashErrorKind::NotAligned,
                esp_storage::FlashStorageError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
                _ => NorFlashErrorKind::Other,
            },
            Esp32FlashError::NotInitialized => NorFlashErrorKind::Other,
        }
    }
}

pub struct Esp32OTA {
    target_partition: u8,
    flash: Option<esp_hal::peripherals::FLASH<'static>>,
}

impl Esp32OTA {
    pub fn new() -> Self {
        Self {
            target_partition: 0,
            flash: None,
        }
    }

    pub fn with_flash(flash: esp_hal::peripherals::FLASH<'static>) -> Self {
        embassy_futures::block_on(async {
            *FLASH_MUTEX.lock().await = Some(EspFlashWrapper::new(flash));
        });
        Self {
            target_partition: 0,
            flash: None,
        }
    }

    fn get_target_address(&self) -> u32 {
        if self.target_partition == 0 {
            OTA_0_OFFSET
        } else {
            OTA_1_OFFSET
        }
    }

    fn get_partition_size(&self) -> u32 {
        OTA_0_SIZE
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

        if total_size > self.get_partition_size() {
            return Err(OTAError::StorageFull);
        }

        let current = Self::get_current_partition();
        self.target_partition = if current == 0 { 1 } else { 0 };
        OTA_TARGET_PARTITION.store(self.target_partition, Ordering::SeqCst);
        OTA_TOTAL.store(total_size, Ordering::SeqCst);
        OTA_RECEIVED.store(0, Ordering::SeqCst);
        OTA_STATE.store(OTAState::Receiving as u8, Ordering::SeqCst);

        let target_addr = self.get_target_address();
        let sector_count = (total_size + SECTOR_SIZE - 1) / SECTOR_SIZE;
        
        let mut guard = FLASH_MUTEX.lock().await;
        let flash = guard.as_mut().ok_or(OTAError::NotInitialized)?;
        
        for sector in 0..sector_count {
            let sector_addr = target_addr + sector * SECTOR_SIZE;
            SyncNorFlash::erase(&mut flash.inner, sector_addr, sector_addr + SECTOR_SIZE)
                .map_err(|_| OTAError::StorageError)?;
        }

        defmt::info!(
            "OTA started: {} bytes to partition {} at 0x{:X}",
            total_size,
            self.target_partition,
            target_addr
        );
        Ok(())
    }

    async fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), Self::Error> {
        if self.get_state() != OTAState::Receiving {
            return Err(OTAError::NotInProgress);
        }

        let target_addr = self.get_target_address();
        let write_addr = target_addr + offset;

        let mut guard = FLASH_MUTEX.lock().await;
        let flash = guard.as_mut().ok_or(OTAError::NotInitialized)?;

        let aligned_offset = (write_addr / 4) * 4;
        let offset_in_word = (write_addr % 4) as usize;
        
        if offset_in_word != 0 || data.len() % 4 != 0 {
            let mut aligned_data = [0xFFu8; 260];
            let read_addr = aligned_offset;
            
            SyncReadNorFlash::read(&mut flash.inner, read_addr, &mut aligned_data[..4])
                .map_err(|_| OTAError::StorageError)?;
            
            let write_len = ((offset_in_word + data.len() + 3) / 4) * 4;
            aligned_data[offset_in_word..offset_in_word + data.len()].copy_from_slice(data);
            
            for chunk_start in (0..write_len).step_by(4) {
                let chunk_addr = read_addr + chunk_start as u32;
                SyncNorFlash::write(&mut flash.inner, chunk_addr, &aligned_data[chunk_start..chunk_start + 4])
                    .map_err(|_| OTAError::WriteFailed)?;
            }
        } else {
            for chunk_start in (0..data.len()).step_by(4) {
                let chunk_addr = write_addr + chunk_start as u32;
                let chunk_end = (chunk_start + 4).min(data.len());
                let chunk = &data[chunk_start..chunk_end];
                
                if chunk.len() == 4 {
                    SyncNorFlash::write(&mut flash.inner, chunk_addr, chunk)
                        .map_err(|_| OTAError::WriteFailed)?;
                } else {
                    let mut padded = [0xFFu8; 4];
                    padded[..chunk.len()].copy_from_slice(chunk);
                    SyncNorFlash::write(&mut flash.inner, chunk_addr, &padded)
                        .map_err(|_| OTAError::WriteFailed)?;
                }
            }
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
        
        let mut guard = FLASH_MUTEX.lock().await;
        let flash = guard.as_mut().ok_or(OTAError::NotInitialized)?;
        
        let mut ota_data = [0u8; 32];
        SyncReadNorFlash::read(&mut flash.inner, OTA_STATE_OFFSET, &mut ota_data)
            .map_err(|_| OTAError::StorageError)?;
        
        ota_data[0] = partition;
        ota_data[1] = 0;
        
        SyncNorFlash::erase(&mut flash.inner, OTA_STATE_OFFSET, OTA_STATE_OFFSET + 4096)
            .map_err(|_| OTAError::StorageError)?;
        SyncNorFlash::write(&mut flash.inner, OTA_STATE_OFFSET, &ota_data)
            .map_err(|_| OTAError::StorageError)?;

        OTA_STATE.store(OTAState::Idle as u8, Ordering::SeqCst);
        defmt::info!(
            "OTA partition {} marked as valid, will boot on next reset",
            partition
        );
        Ok(())
    }

    fn get_ota_partition_size(&self) -> u32 {
        self.get_partition_size()
    }
}