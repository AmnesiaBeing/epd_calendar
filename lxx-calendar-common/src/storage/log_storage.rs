//! Log Storage Module
//!
//! Provides persistent log storage with circular buffer implementation.
//! Features:
//! - Circular buffer for log entries
//! - Wear leveling across sectors
//! - Automatic wrap-around when storage is full
//! - Timestamp and log level support

use crate::SystemResult;
use crate::flash_layout::{
    LOG_MAGIC, LOG_MAX_ENTRY_SIZE, LOG_OFFSET, LOG_SECTOR_COUNT, LOG_SIZE, SECTOR_SIZE,
};
use crate::types::error::{StorageError, SystemError};
use core::mem::size_of;

use crate::{debug, info};

#[cfg(feature = "defmt")]
use defmt::Format;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(LogLevel::Error),
            1 => Some(LogLevel::Warn),
            2 => Some(LogLevel::Info),
            3 => Some(LogLevel::Debug),
            4 => Some(LogLevel::Trace),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LogEntryHeader {
    magic: u32,
    timestamp: u32,
    level: u8,
    length: u8,
    checksum: u16,
}

impl LogEntryHeader {
    pub const SIZE: usize = size_of::<Self>();

    pub fn new(timestamp: u32, level: LogLevel, length: u8) -> Self {
        let header = Self {
            magic: LOG_MAGIC,
            timestamp,
            level: level.as_u8(),
            length,
            checksum: 0,
        };
        let checksum = header.calculate_checksum();
        Self { checksum, ..header }
    }

    fn calculate_checksum(&self) -> u16 {
        let data = [
            self.magic.to_le_bytes()[0],
            self.magic.to_le_bytes()[1],
            self.magic.to_le_bytes()[2],
            self.magic.to_le_bytes()[3],
            self.timestamp.to_le_bytes()[0],
            self.timestamp.to_le_bytes()[1],
            self.timestamp.to_le_bytes()[2],
            self.timestamp.to_le_bytes()[3],
            self.level,
            self.length,
        ];
        let mut sum: u16 = 0;
        for &byte in &data {
            sum = sum.wrapping_add(byte as u16);
        }
        sum
    }

    pub fn verify(&self) -> bool {
        self.magic == LOG_MAGIC && self.checksum == self.calculate_checksum()
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.timestamp.to_le_bytes());
        bytes[8] = self.level;
        bytes[9] = self.length;
        bytes[10..12].copy_from_slice(&self.checksum.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let header = Self {
            magic: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            timestamp: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            level: bytes[8],
            length: bytes[9],
            checksum: u16::from_le_bytes([bytes[10], bytes[11]]),
        };
        if header.verify() { Some(header) } else { None }
    }

    pub fn level(&self) -> Option<LogLevel> {
        LogLevel::from_u8(self.level)
    }

    pub fn length(&self) -> usize {
        self.length as usize
    }

    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct LogEntry {
    pub timestamp: u32,
    pub level: LogLevel,
    pub message: [u8; LOG_MAX_ENTRY_SIZE],
    pub message_len: usize,
}

impl LogEntry {
    pub fn new(timestamp: u32, level: LogLevel, message: &[u8]) -> Self {
        let len = message.len().min(LOG_MAX_ENTRY_SIZE);
        let mut msg_buf = [0u8; LOG_MAX_ENTRY_SIZE];
        msg_buf[..len].copy_from_slice(&message[..len]);
        Self {
            timestamp,
            level,
            message: msg_buf,
            message_len: len,
        }
    }

    pub fn message_str(&self) -> &str {
        core::str::from_utf8(&self.message[..self.message_len]).unwrap_or("<invalid utf8>")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogIteratorState {
    SeekStart,
    Reading,
    Done,
}

pub struct LogIterator<'a, F: FlashDevice> {
    storage: &'a mut LogStorage<F>,
    current_sector: u32,
    current_offset: u32,
    state: LogIteratorState,
}

impl<'a, F: FlashDevice> LogIterator<'a, F> {
    pub fn new(storage: &'a mut LogStorage<F>) -> Self {
        Self {
            storage,
            current_sector: 0,
            current_offset: 0,
            state: LogIteratorState::SeekStart,
        }
    }

    pub async fn next(&mut self) -> SystemResult<Option<LogEntry>> {
        loop {
            match self.state {
                LogIteratorState::SeekStart => {
                    self.current_sector = 0;
                    self.current_offset = 0;
                    self.state = LogIteratorState::Reading;
                }
                LogIteratorState::Reading => {
                    if self.current_sector >= LOG_SECTOR_COUNT {
                        self.state = LogIteratorState::Done;
                        continue;
                    }

                    let offset =
                        LOG_OFFSET + self.current_sector * SECTOR_SIZE + self.current_offset;

                    if self.current_offset >= SECTOR_SIZE {
                        self.current_sector += 1;
                        self.current_offset = 0;
                        continue;
                    }

                    let mut header_buf = [0u8; LogEntryHeader::SIZE];
                    match self.storage.flash.read(offset, &mut header_buf).await {
                        Ok(_) => {}
                        Err(_) => {
                            self.current_sector += 1;
                            self.current_offset = 0;
                            continue;
                        }
                    }

                    if let Some(header) = LogEntryHeader::from_bytes(&header_buf) {
                        if header.magic != LOG_MAGIC {
                            self.current_offset += 1;
                            continue;
                        }

                        let total_size = LogEntryHeader::SIZE + header.length();
                        let mut entry_buf = [0u8; LOG_MAX_ENTRY_SIZE + LogEntryHeader::SIZE];

                        if header.length() > 0 {
                            let data_offset = offset + LogEntryHeader::SIZE as u32;
                            self.storage
                                .flash
                                .read(data_offset, &mut entry_buf[..header.length()])
                                .await?;
                        }

                        self.current_offset += total_size as u32;
                        if self.current_offset % 4 != 0 {
                            self.current_offset = (self.current_offset / 4 + 1) * 4;
                        }

                        let level = header.level().unwrap_or(LogLevel::Info);
                        let entry =
                            LogEntry::new(header.timestamp(), level, &entry_buf[..header.length()]);

                        return Ok(Some(entry));
                    } else {
                        self.current_offset += 4;
                    }
                }
                LogIteratorState::Done => {
                    return Ok(None);
                }
            }
        }
    }
}

use super::FlashDevice;

pub struct LogStorage<F: FlashDevice> {
    flash: F,
    write_sector: u32,
    write_offset: u32,
    initialized: bool,
}

impl<F: FlashDevice> LogStorage<F> {
    pub fn new(flash: F) -> Self {
        Self {
            flash,
            write_sector: 0,
            write_offset: 0,
            initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        self.find_write_position().await?;
        self.initialized = true;
        info!("Log storage initialized at sector {}", self.write_sector);
        Ok(())
    }

    async fn find_write_position(&mut self) -> SystemResult<()> {
        let mut header_buf = [0u8; LogEntryHeader::SIZE];

        for sector in 0..LOG_SECTOR_COUNT {
            let sector_offset = LOG_OFFSET + sector * SECTOR_SIZE;
            let mut found_empty = false;

            for offset in (0..SECTOR_SIZE as usize).step_by(4) {
                let addr = sector_offset + offset as u32;
                self.flash.read(addr, &mut header_buf).await?;

                if header_buf[0..4] == [0xFF, 0xFF, 0xFF, 0xFF] {
                    self.write_sector = sector;
                    self.write_offset = offset as u32;
                    found_empty = true;
                    break;
                }
            }

            if found_empty {
                return Ok(());
            }
        }

        self.write_sector = 0;
        self.write_offset = 0;
        Ok(())
    }

    pub async fn write(
        &mut self,
        timestamp: u32,
        level: LogLevel,
        message: &[u8],
    ) -> SystemResult<()> {
        if !self.initialized {
            return Err(SystemError::StorageError(StorageError::WriteFailed));
        }

        let msg_len = message.len().min(LOG_MAX_ENTRY_SIZE);
        let total_size = (LogEntryHeader::SIZE + msg_len + 3) & !3;

        if self.write_offset + total_size as u32 > SECTOR_SIZE {
            self.advance_sector().await?;
        }

        let header = LogEntryHeader::new(timestamp, level, msg_len as u8);
        let header_bytes = header.to_bytes();

        let write_addr = LOG_OFFSET + self.write_sector * SECTOR_SIZE + self.write_offset;

        self.flash.write(write_addr, &header_bytes).await?;

        if msg_len > 0 {
            let mut aligned_msg = [0u8; LOG_MAX_ENTRY_SIZE + 4];
            aligned_msg[..msg_len].copy_from_slice(&message[..msg_len]);
            let aligned_len = (msg_len + 3) & !3;
            self.flash
                .write(
                    write_addr + LogEntryHeader::SIZE as u32,
                    &aligned_msg[..aligned_len],
                )
                .await?;
        }

        self.write_offset += total_size as u32;

        Ok(())
    }

    async fn advance_sector(&mut self) -> SystemResult<()> {
        self.write_sector = (self.write_sector + 1) % LOG_SECTOR_COUNT;
        self.write_offset = 0;

        let sector_start = LOG_OFFSET + self.write_sector * SECTOR_SIZE;
        self.flash
            .erase(sector_start, sector_start + SECTOR_SIZE)
            .await?;

        debug!("Log storage advanced to sector {}", self.write_sector);
        Ok(())
    }

    pub async fn clear(&mut self) -> SystemResult<()> {
        for sector in 0..LOG_SECTOR_COUNT {
            let sector_start = LOG_OFFSET + sector * SECTOR_SIZE;
            self.flash
                .erase(sector_start, sector_start + SECTOR_SIZE)
                .await?;
        }

        self.write_sector = 0;
        self.write_offset = 0;

        info!("Log storage cleared");
        Ok(())
    }

    pub fn iterator<'a>(&'a mut self) -> LogIterator<'a, F> {
        LogIterator::new(self)
    }

    pub async fn read_entries(
        &mut self,
        max_entries: usize,
    ) -> SystemResult<heapless::Vec<LogEntry, 64>> {
        let mut entries = heapless::Vec::new();
        let mut iter = self.iterator();

        while entries.len() < max_entries {
            match iter.next().await? {
                Some(entry) => {
                    if entries.push(entry).is_err() {
                        break;
                    }
                }
                None => break,
            }
        }

        Ok(entries)
    }

    pub fn stats(&self) -> LogStorageStats {
        LogStorageStats {
            current_sector: self.write_sector,
            current_offset: self.write_offset,
            total_sectors: LOG_SECTOR_COUNT,
            used_bytes: self.write_sector * SECTOR_SIZE + self.write_offset,
            total_bytes: LOG_SIZE,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LogStorageStats {
    pub current_sector: u32,
    pub current_offset: u32,
    pub total_sectors: u32,
    pub used_bytes: u32,
    pub total_bytes: u32,
}

impl LogStorageStats {
    pub fn usage_percent(&self) -> u8 {
        ((self.used_bytes as u64 * 100 / self.total_bytes as u64) as u8).min(100)
    }
}
