//! Flash Partition Layout Constants
//!
//! This module defines the flash memory layout for ESP32-C6.
//! The layout is designed to support:
//! - Configuration storage with wear-leveling (dual-bank)
//! - Circular log storage
//! - OTA updates (A/B partitions)
//!
//! ## Memory Map (4MB Flash)
//!
//! ```text
//! ┌─────────────────┬───────────┬─────────────┬──────────────────────┐
//! │ Partition       │ Offset    │ Size        │ Description          │
//! ├─────────────────┼───────────┼─────────────┼──────────────────────┤
//! │ Bootloader      │ 0x00000   │ 28KB        │ ESP-IDF bootloader   │
//! │ Partition Table │ 0x08000   │ 4KB         │ This file            │
//! │ NVS             │ 0x09000   │ 24KB        │ WiFi/system config   │
//! │ PHY Init        │ 0x0F000   │ 4KB         │ RF calibration       │
//! │ App Config A    │ 0x10000   │ 8KB         │ Primary config       │
//! │ App Config B    │ 0x12000   │ 8KB         │ Backup config        │
//! │ Log Storage     │ 0x14000   │ 48KB        │ Circular log buffer  │
//! │ Factory App     │ 0x20000   │ 1MB         │ Factory firmware     │
//! │ OTA_0           │ 0x120000  │ 1MB         │ OTA partition 0      │
//! │ OTA_1           │ 0x220000  │ 1MB         │ OTA partition 1      │
//! │ OTA State       │ 0x320000  │ 8KB         │ OTA boot state       │
//! │ Reserved        │ 0x322000  │ ~888KB      │ Future use           │
//! └─────────────────┴───────────┴─────────────┴──────────────────────┘
//! ```

#![allow(dead_code)]

pub const FLASH_SIZE: u32 = 4 * 1024 * 1024;
pub const SECTOR_SIZE: u32 = 4096;
pub const PAGE_SIZE: u32 = 256;

// ============================================================================
// Bootloader Region
// ============================================================================

pub const BOOTLOADER_OFFSET: u32 = 0x00000;
pub const BOOTLOADER_SIZE: u32 = 28 * 1024;

pub const PARTITION_TABLE_OFFSET: u32 = 0x08000;
pub const PARTITION_TABLE_SIZE: u32 = 4 * 1024;

// ============================================================================
// System Partitions
// ============================================================================

pub const NVS_OFFSET: u32 = 0x09000;
pub const NVS_SIZE: u32 = 24 * 1024;

pub const PHY_INIT_OFFSET: u32 = 0x0F000;
pub const PHY_INIT_SIZE: u32 = 4 * 1024;

// ============================================================================
// Application Configuration (Dual-bank for wear leveling)
// ============================================================================

pub const CONFIG_A_OFFSET: u32 = 0x10000;
pub const CONFIG_A_SIZE: u32 = 8 * 1024;

pub const CONFIG_B_OFFSET: u32 = 0x12000;
pub const CONFIG_B_SIZE: u32 = 8 * 1024;

pub const CONFIG_SIZE: u32 = CONFIG_A_SIZE;
pub const CONFIG_HEADER_SIZE: usize = 32;
pub const CONFIG_MAX_DATA_SIZE: usize = 1024;

// ============================================================================
// Log Storage (Circular buffer)
// ============================================================================

pub const LOG_OFFSET: u32 = 0x14000;
pub const LOG_SIZE: u32 = 48 * 1024;
pub const LOG_SECTOR_COUNT: u32 = LOG_SIZE / SECTOR_SIZE;

pub const LOG_ENTRY_HEADER_SIZE: usize = 8;
pub const LOG_MAX_ENTRY_SIZE: usize = 256;
pub const LOG_MAGIC: u32 = 0x4C4F4745; // "LOGE"

// ============================================================================
// Application Partitions
// ============================================================================

pub const FACTORY_APP_OFFSET: u32 = 0x20000;
pub const FACTORY_APP_SIZE: u32 = 1024 * 1024;

pub const OTA_0_OFFSET: u32 = 0x120000;
pub const OTA_0_SIZE: u32 = 1024 * 1024;

pub const OTA_1_OFFSET: u32 = 0x220000;
pub const OTA_1_SIZE: u32 = 1024 * 1024;

pub const OTA_STATE_OFFSET: u32 = 0x320000;
pub const OTA_STATE_SIZE: u32 = 8 * 1024;

// ============================================================================
// Reserved Region
// ============================================================================

pub const RESERVED_OFFSET: u32 = 0x322000;
pub const RESERVED_SIZE: u32 = FLASH_SIZE - RESERVED_OFFSET;

// ============================================================================
// Helper Functions
// ============================================================================

#[inline]
pub const fn align_to_sector(offset: u32) -> u32 {
    (offset / SECTOR_SIZE) * SECTOR_SIZE
}

#[inline]
pub const fn is_aligned_to_sector(offset: u32) -> bool {
    offset % SECTOR_SIZE == 0
}

#[inline]
pub const fn sector_count(size: u32) -> u32 {
    (size + SECTOR_SIZE - 1) / SECTOR_SIZE
}
