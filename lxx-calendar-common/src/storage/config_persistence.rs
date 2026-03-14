//! Configuration Persistence with Wear Leveling
//!
//! Uses dual-bank configuration storage for wear leveling.
//! Each bank stores configuration with:
//! - Magic number for identification
//! - Version for compatibility check
//! - CRC32 checksum for data integrity
//! - Active flag to determine which bank is current
//!
//! On each save, the inactive bank is written first,
//! then the active flag is switched.

use crate::SystemResult;
use crate::flash_layout::{
    CONFIG_A_OFFSET, CONFIG_A_SIZE, CONFIG_B_OFFSET, CONFIG_B_SIZE, CONFIG_HEADER_SIZE,
    CONFIG_MAX_DATA_SIZE, SECTOR_SIZE,
};
use crate::types::error::{StorageError, SystemError};
use crate::{info, warn};

use embedded_storage_async::nor_flash::{NorFlash, ReadNorFlash};
use postcard;
use serde::{Deserialize, Serialize};

const CONFIG_VERSION: u32 = 1;
const CONFIG_MAGIC: u32 = 0x4C585843; // "LXXC" in little endian
const ACTIVE_FLAG: u32 = 0x41435456; // "ACTV" - active bank marker
const INACTIVE_FLAG: u32 = 0xFFFFFFFF; // inactive bank marker

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ConfigHeader {
    magic: u32,
    version: u32,
    checksum: u32,
    active: u32,
    reserved: [u8; 16],
}

impl ConfigHeader {
    pub const SIZE: usize = CONFIG_HEADER_SIZE;

    pub fn new(checksum: u32, active: bool) -> Self {
        Self {
            magic: CONFIG_MAGIC,
            version: CONFIG_VERSION,
            checksum,
            active: if active { ACTIVE_FLAG } else { INACTIVE_FLAG },
            reserved: [0; 16],
        }
    }

    pub fn is_active(&self) -> bool {
        self.active == ACTIVE_FLAG
    }

    pub fn is_valid(&self) -> bool {
        self.magic == CONFIG_MAGIC && self.version == CONFIG_VERSION
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        let mut temp_buf = [0u8; Self::SIZE];
        if let Ok(serialized) = postcard::to_slice(self, &mut temp_buf[..Self::SIZE - 4]) {
            buf[..serialized.len()].copy_from_slice(serialized);
        }
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }
}

pub trait FlashDevice {
    async fn read(&mut self, offset: u32, buf: &mut [u8]) -> SystemResult<()>;
    async fn write(&mut self, offset: u32, buf: &[u8]) -> SystemResult<()>;
    async fn erase(&mut self, from: u32, to: u32) -> SystemResult<()>;
    fn sector_size(&self) -> u32;
}

impl<F> FlashDevice for F
where
    F: NorFlash,
{
    async fn read(&mut self, offset: u32, buf: &mut [u8]) -> SystemResult<()> {
        ReadNorFlash::read(self, offset, buf)
            .await
            .map_err(|_| SystemError::StorageError(StorageError::ReadFailed))
    }

    async fn write(&mut self, offset: u32, buf: &[u8]) -> SystemResult<()> {
        NorFlash::write(self, offset, buf)
            .await
            .map_err(|_| SystemError::StorageError(StorageError::WriteFailed))
    }

    async fn erase(&mut self, from: u32, to: u32) -> SystemResult<()> {
        NorFlash::erase(self, from, to)
            .await
            .map_err(|_| SystemError::StorageError(StorageError::WriteFailed))
    }

    fn sector_size(&self) -> u32 {
        SECTOR_SIZE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigBank {
    A,
    B,
}

pub struct ConfigPersistence<F: FlashDevice> {
    flash: F,
    active_bank: Option<ConfigBank>,
}

impl<F: FlashDevice> ConfigPersistence<F> {
    pub fn new(flash: F) -> Self {
        Self {
            flash,
            active_bank: None,
        }
    }

    #[deprecated(note = "Use new() without offset parameter, layout is fixed")]
    pub fn with_offset(flash: F, _offset: u32) -> Self {
        Self::new(flash)
    }

    fn calculate_checksum(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFFFFFF;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    fn bank_offset(bank: ConfigBank) -> u32 {
        match bank {
            ConfigBank::A => CONFIG_A_OFFSET,
            ConfigBank::B => CONFIG_B_OFFSET,
        }
    }

    fn bank_size(bank: ConfigBank) -> u32 {
        match bank {
            ConfigBank::A => CONFIG_A_SIZE,
            ConfigBank::B => CONFIG_B_SIZE,
        }
    }

    async fn read_bank_header(&mut self, bank: ConfigBank) -> SystemResult<ConfigHeader> {
        let offset = Self::bank_offset(bank);
        let mut header_buf = [0u8; CONFIG_HEADER_SIZE];
        self.flash.read(offset, &mut header_buf).await?;

        ConfigHeader::from_bytes(&header_buf)
            .ok_or(SystemError::StorageError(StorageError::Corrupted))
    }

    async fn determine_active_bank(&mut self) -> SystemResult<ConfigBank> {
        if let Some(bank) = self.active_bank {
            return Ok(bank);
        }

        let header_a = self.read_bank_header(ConfigBank::A).await;
        let header_b = self.read_bank_header(ConfigBank::B).await;

        let bank = match (header_a, header_b) {
            (Ok(ha), Ok(hb)) => {
                if ha.is_active() && ha.is_valid() {
                    ConfigBank::A
                } else if hb.is_active() && hb.is_valid() {
                    ConfigBank::B
                } else if ha.is_valid() {
                    ConfigBank::A
                } else if hb.is_valid() {
                    ConfigBank::B
                } else {
                    ConfigBank::A
                }
            }
            (Ok(ha), Err(_)) => {
                if ha.is_valid() {
                    ConfigBank::A
                } else {
                    ConfigBank::A
                }
            }
            (Err(_), Ok(hb)) => {
                if hb.is_valid() {
                    ConfigBank::B
                } else {
                    ConfigBank::A
                }
            }
            (Err(_), Err(_)) => ConfigBank::A,
        };

        self.active_bank = Some(bank);
        Ok(bank)
    }

    pub async fn load_config<T>(&mut self) -> SystemResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let bank = self.determine_active_bank().await?;
        let offset = Self::bank_offset(bank);

        let mut header_buf = [0u8; CONFIG_HEADER_SIZE];
        self.flash.read(offset, &mut header_buf).await?;

        let header = ConfigHeader::from_bytes(&header_buf)
            .ok_or(SystemError::StorageError(StorageError::Corrupted))?;

        if !header.is_valid() {
            info!("Config header invalid, using default");
            return Err(SystemError::StorageError(StorageError::Corrupted));
        }

        if header.version != CONFIG_VERSION {
            info!(
                "Config version mismatch (stored={}, expected={})",
                header.version, CONFIG_VERSION
            );
            return Err(SystemError::StorageError(StorageError::Corrupted));
        }

        let mut data_buf = [0u8; CONFIG_MAX_DATA_SIZE];
        self.flash
            .read(offset + CONFIG_HEADER_SIZE as u32, &mut data_buf)
            .await?;

        if header.checksum != Self::calculate_checksum(&data_buf) {
            warn!("Config checksum mismatch");
            return Err(SystemError::StorageError(StorageError::Corrupted));
        }

        postcard::from_bytes(&data_buf)
            .map_err(|_| SystemError::StorageError(StorageError::Corrupted))
    }

    pub async fn save_config<T>(&mut self, config: &T) -> SystemResult<()>
    where
        T: Serialize,
    {
        let mut buf = [0u8; CONFIG_MAX_DATA_SIZE];
        let serialized = postcard::to_slice(config, &mut buf)
            .map_err(|_| SystemError::StorageError(StorageError::WriteFailed))?;
        let serialized_len = serialized.len();

        let checksum = Self::calculate_checksum(&buf[..serialized_len]);

        let target_bank = match self.active_bank {
            Some(ConfigBank::A) => ConfigBank::B,
            Some(ConfigBank::B) => ConfigBank::A,
            None => ConfigBank::A,
        };

        let offset = Self::bank_offset(target_bank);
        let size = Self::bank_size(target_bank);

        self.flash.erase(offset, offset + size).await?;

        let header = ConfigHeader::new(checksum, true);
        let header_buf = header.to_bytes();
        self.flash.write(offset, &header_buf).await?;

        self.flash
            .write(offset + CONFIG_HEADER_SIZE as u32, &buf)
            .await?;

        if let Some(old_bank) = self.active_bank {
            let old_offset = Self::bank_offset(old_bank);
            let mut old_header_buf = [0u8; CONFIG_HEADER_SIZE];
            self.flash.read(old_offset, &mut old_header_buf).await?;

            if let Some(mut old_header) = ConfigHeader::from_bytes(&old_header_buf) {
                old_header.active = INACTIVE_FLAG;
                let updated_header_buf = old_header.to_bytes();
                self.flash.write(old_offset, &updated_header_buf).await?;
            }
        }

        self.active_bank = Some(target_bank);
        info!("Config saved to bank {:?}", target_bank);

        Ok(())
    }

    pub async fn factory_reset(&mut self) -> SystemResult<()> {
        self.flash
            .erase(CONFIG_A_OFFSET, CONFIG_A_OFFSET + CONFIG_A_SIZE)
            .await?;
        self.flash
            .erase(CONFIG_B_OFFSET, CONFIG_B_OFFSET + CONFIG_B_SIZE)
            .await?;

        self.active_bank = None;
        info!("Factory reset completed");
        Ok(())
    }

    pub async fn config_exists(&mut self) -> bool {
        let bank = self.determine_active_bank().await.ok();
        if let Some(bank) = bank {
            let offset = Self::bank_offset(bank);
            let mut magic_buf = [0u8; 4];
            if self.flash.read(offset, &mut magic_buf).await.is_ok() {
                let magic = u32::from_le_bytes(magic_buf);
                return magic == CONFIG_MAGIC;
            }
        }
        false
    }

    pub fn get_active_bank(&self) -> Option<ConfigBank> {
        self.active_bank
    }

    pub async fn validate_bank(&mut self, bank: ConfigBank) -> SystemResult<bool> {
        let header = self.read_bank_header(bank).await?;
        Ok(header.is_valid() && header.is_active())
    }
}
