use embedded_storage_async::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};
use std::path::PathBuf;
use std::sync::Mutex;

const FLASH_SIZE: usize = 4 * 1024 * 1024;
const SECTOR_SIZE: usize = 4096;

#[derive(Debug)]
pub enum FlashError {
    IoError(std::io::Error),
    OutOfBounds,
}

impl From<std::io::Error> for FlashError {
    fn from(e: std::io::Error) -> Self {
        FlashError::IoError(e)
    }
}

impl NorFlashError for FlashError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            FlashError::IoError(_) => NorFlashErrorKind::Other,
            FlashError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
        }
    }
}

pub struct SimulatedFlash {
    data: Mutex<Vec<u8>>,
    path: PathBuf,
}

impl SimulatedFlash {
    pub fn new(path: PathBuf) -> Self {
        let data = if path.exists() {
            std::fs::read(&path).unwrap_or_else(|_| vec![0xFFu8; FLASH_SIZE])
        } else {
            let initial_data = vec![0xFFu8; FLASH_SIZE];
            std::fs::write(&path, &initial_data).ok();
            initial_data
        };

        Self {
            data: Mutex::new(data),
            path,
        }
    }

    pub fn new_with_config(
        path: PathBuf,
        size: usize,
        _erase_size: usize,
        _write_size: usize,
    ) -> Self {
        let data = if path.exists() {
            std::fs::read(&path).unwrap_or_else(|_| vec![0xFFu8; size])
        } else {
            let initial_data = vec![0xFFu8; size];
            std::fs::write(&path, &initial_data).ok();
            initial_data
        };

        Self {
            data: Mutex::new(data),
            path,
        }
    }

    fn save_to_disk(&self) -> Result<(), FlashError> {
        let data = self.data.lock().unwrap();
        std::fs::write(&self.path, data.as_slice())?;
        Ok(())
    }

    pub fn get_config_a_offset(&self) -> u32 {
        lxx_calendar_common::flash_layout::CONFIG_A_OFFSET
    }

    pub fn get_config_b_offset(&self) -> u32 {
        lxx_calendar_common::flash_layout::CONFIG_B_OFFSET
    }

    pub fn get_log_offset(&self) -> u32 {
        lxx_calendar_common::flash_layout::LOG_OFFSET
    }
}

impl ErrorType for SimulatedFlash {
    type Error = FlashError;
}

impl NorFlash for SimulatedFlash {
    const WRITE_SIZE: usize = 4;
    const ERASE_SIZE: usize = SECTOR_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let mut data = self.data.lock().unwrap();
        for addr in from..to.min(data.len() as u32) {
            data[addr as usize] = 0xFF;
        }
        drop(data);
        self.save_to_disk()?;
        Ok(())
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut data = self.data.lock().unwrap();
        let offset = offset as usize;
        let end = offset + bytes.len();
        if end > data.len() {
            return Err(FlashError::OutOfBounds);
        }
        data[offset..end].copy_from_slice(bytes);
        drop(data);
        self.save_to_disk()?;
        Ok(())
    }
}

impl ReadNorFlash for SimulatedFlash {
    const READ_SIZE: usize = 1;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let data = self.data.lock().unwrap();
        let offset = offset as usize;
        let end = offset + bytes.len();
        if end > data.len() {
            return Err(FlashError::OutOfBounds);
        }
        bytes.copy_from_slice(&data[offset..end]);
        Ok(())
    }

    fn capacity(&self) -> usize {
        FLASH_SIZE
    }
}
