use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_storage::nor_flash::{NorFlash as SyncNorFlash, ReadNorFlash as SyncReadNorFlash};
use embedded_storage_async::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};
use esp_storage::{FlashStorage, FlashStorageError};

static FLASH_MUTEX: Mutex<CriticalSectionRawMutex, Option<FlashStorage<'static>>> =
    Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Esp32FlashError {
    Flash(FlashStorageError),
    NotInitialized,
}

impl NorFlashError for Esp32FlashError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Esp32FlashError::Flash(e) => match e {
                FlashStorageError::NotAligned => NorFlashErrorKind::NotAligned,
                FlashStorageError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
                _ => NorFlashErrorKind::Other,
            },
            Esp32FlashError::NotInitialized => NorFlashErrorKind::Other,
        }
    }
}

pub struct Esp32Flash;

impl Esp32Flash {
    pub fn new(flash: esp_hal::peripherals::FLASH<'static>) -> Self {
        let storage = FlashStorage::new(flash);
        embassy_futures::block_on(async {
            *FLASH_MUTEX.lock().await = Some(storage);
        });
        Self
    }
}

impl ErrorType for Esp32Flash {
    type Error = Esp32FlashError;
}

impl ReadNorFlash for Esp32Flash {
    const READ_SIZE: usize = 4;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let mut guard = FLASH_MUTEX.lock().await;
        let flash = guard.as_mut().ok_or(Esp32FlashError::NotInitialized)?;
        SyncReadNorFlash::read(flash, offset, bytes).map_err(Esp32FlashError::Flash)
    }

    fn capacity(&self) -> usize {
        4 * 1024 * 1024
    }
}

impl NorFlash for Esp32Flash {
    const WRITE_SIZE: usize = 4;
    const ERASE_SIZE: usize = 4096;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let mut guard = FLASH_MUTEX.lock().await;
        let flash = guard.as_mut().ok_or(Esp32FlashError::NotInitialized)?;
        SyncNorFlash::erase(flash, from, to).map_err(Esp32FlashError::Flash)
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut guard = FLASH_MUTEX.lock().await;
        let flash = guard.as_mut().ok_or(Esp32FlashError::NotInitialized)?;
        SyncNorFlash::write(flash, offset, bytes).map_err(Esp32FlashError::Flash)
    }
}
