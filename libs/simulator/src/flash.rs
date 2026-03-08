use embedded_storage_async::nor_flash::{ErrorType, NorFlashErrorKind, NorFlash, NorFlashError, ReadNorFlash};
use std::path::PathBuf;
use std::sync::Mutex;

/// Flash 错误类型
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

/// 模拟器用的文件模拟Flash存储
pub struct SimulatedFlash {
    data: Mutex<Vec<u8>>,
    path: PathBuf,
    write_size: usize,
    erase_size: usize,
    size: usize,
}

impl SimulatedFlash {
    pub fn new(path: PathBuf, size: usize, erase_size: usize, write_size: usize) -> Self {
        // 尝试从文件加载数据，如果文件不存在则创建新的
        let data = if path.exists() {
            std::fs::read(&path).unwrap_or_else(|_| vec![0xFFu8; size])
        } else {
            // 创建初始数据（全0xFF，表示未写入的Flash）
            let initial_data = vec![0xFFu8; size];
            // 立即写入文件以确保文件被创建
            std::fs::write(&path, &initial_data).ok();
            initial_data
        };

        Self {
            data: Mutex::new(data),
            path,
            write_size,
            erase_size,
            size,
        }
    }

    fn save_to_disk(&self) -> Result<(), FlashError> {
        let data = self.data.lock().unwrap();
        std::fs::write(&self.path, data.as_slice())?;
        Ok(())
    }
}

impl ErrorType for SimulatedFlash {
    type Error = FlashError;
}

impl NorFlash for SimulatedFlash {
    const WRITE_SIZE: usize = 4096;
    const ERASE_SIZE: usize = 4096;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let mut data = self.data.lock().unwrap();
        for addr in from..to.min(data.len() as u32) {
            data[addr as usize] = 0xFF;
        }
        drop(data); // 释放锁
        self.save_to_disk()?;
        Ok(())
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut data = self.data.lock().unwrap();
        let offset = offset as usize;
        data[offset..offset + bytes.len()].copy_from_slice(bytes);
        drop(data); // 释放锁
        self.save_to_disk()?;
        Ok(())
    }
}

impl ReadNorFlash for SimulatedFlash {
    const READ_SIZE: usize = 1;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let data = self.data.lock().unwrap();
        let offset = offset as usize;
        bytes.copy_from_slice(&data[offset..offset + bytes.len()]);
        Ok(())
    }

    fn capacity(&self) -> usize {
        self.size
    }
}