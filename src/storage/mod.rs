//! A storage abstraction layer for embedded systems
//!
//! This crate provides a comprehensive set of traits and implementations for working with
//! different types of storage devices in embedded systems. It includes traits for both
//! synchronous and asynchronous operations, as well as support for various storage technologies.
//!

#![allow(missing_docs)]
#![allow(async_fn_in_trait)]
#![deny(unsafe_code)]

/// Common error types for storage operations
pub mod error;

/// Re-exports of common traits
pub mod prelude {
    #[cfg(feature = "async")]
    pub use super::{
        AsyncBlockStorage, AsyncErase, AsyncReadStorage, AsyncSectorStorage, AsyncStorage,
    };
    pub use super::{BlockStorage, BlockingErase, ReadStorage, Region, SectorStorage, Storage};
}

/// A contiguous memory region
pub trait Region {
    /// Start address of the region
    fn start(&self) -> u32;

    /// End address of the region (exclusive)
    fn end(&self) -> u32;

    /// Check if address is contained in the region
    fn contains(&self, address: u32) -> bool {
        (address >= self.start()) && (address < self.end())
    }
}

// Core synchronous traits (unchanged from previous)
pub trait ReadStorage {
    /// Associated error type
    type Error: core::fmt::Debug;

    /// Read data from the storage
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;

    /// Get the total capacity of the storage
    fn capacity(&self) -> usize;
}
pub trait Storage: ReadStorage {
    /// Write data to the storage
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;
}
pub trait BlockingErase: Storage {
    /// Erase a region of the storage. The erased bytes should be `0xFF`.
    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error>;
}

// Core async traits (unchanged from previous)
#[cfg(feature = "async")]
pub trait AsyncReadStorage {
    /// Associated error type
    type Error: core::fmt::Debug;

    /// Read data from the storage asynchronously
    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;

    /// Get the total capacity of the storage
    fn capacity(&self) -> usize;
}
#[cfg(feature = "async")]
pub trait AsyncStorage: AsyncReadStorage {
    /// Write data to the storage asynchronously
    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;
}
#[cfg(feature = "async")]
pub trait AsyncErase: AsyncStorage {
    /// Erase a region of the storage asynchronously
    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error>;
}

/// ======================
/// Technology-Specific Extensions
/// ======================

/// EEPROM-specific operations
pub trait Eeprom: Storage {
    /// Write a byte with verification
    fn write_verified(&mut self, offset: u32, byte: u8)
        -> Result<(), <Self as ReadStorage>::Error>;

    /// Get page size (if applicable)
    fn page_size(&self) -> Option<usize>;
}

/// SD/MMC card operations
pub trait SdMmc: Storage + BlockStorage + SectorStorage {
    /// Initialize the card
    fn init(&mut self) -> Result<(), <Self as ReadStorage>::Error>;

    /// Get card status
    fn status(&mut self) -> Result<SdMmcStatus, <Self as ReadStorage>::Error>;

    /// Set block length (for SDSC cards)
    fn set_block_length(&mut self, len: u32) -> Result<(), <Self as ReadStorage>::Error>;
}

/// SD/MMC card status information
pub struct SdMmcStatus {
    pub initialized: bool,
    pub capacity: u64,
    pub protection: bool,
    // ... other fields
}

/// RAM-based storage operations
pub trait RamStorage: Storage {
    /// Clear entire storage (volatile only)
    fn clear(&mut self) -> Result<(), <Self as ReadStorage>::Error>;

    /// Get underlying memory reference
    fn as_slice(&self) -> &[u8];

    /// Get mutable underlying memory reference
    fn as_mut_slice(&mut self) -> &mut [u8];
}

/// NAND Flash operations
pub trait NandFlash: Storage + BlockStorage {
    /// Read spare area data
    fn read_spare(
        &mut self,
        block: usize,
        page: usize,
        spare: &mut [u8],
    ) -> Result<(), <Self as ReadStorage>::Error>;

    /// Write with spare area
    fn write_with_spare(
        &mut self,
        block: usize,
        page: usize,
        data: &[u8],
        spare: &[u8],
    ) -> Result<(), <Self as ReadStorage>::Error>;

    /// Check block status (good/bad)
    fn block_status(&mut self, block: usize) -> Result<BlockStatus, <Self as ReadStorage>::Error>;
}

/// NAND block status
pub enum BlockStatus {
    Good,
    Bad,
    Reserved,
}

/// FRAM (Ferroelectric RAM) operations
pub trait Fram: Storage {
    /// FRAM-specific endurance information
    fn endurance(&self) -> Option<u32>; // in write cycles

    /// FRAM typically doesn't need erase
    fn requires_erase(&self) -> bool {
        false
    }
}

/// ======================
/// Composite Traits
/// ======================

/// Unified storage that can be either volatile or non-volatile
pub trait UnifiedStorage: Storage {
    /// Returns true if storage is non-volatile
    fn is_non_volatile(&self) -> bool;
}

/// Block-oriented storage (common for SD/MMC/NAND)
pub trait BlockStorage {
    fn block_size(&self) -> usize;
    fn block_count(&self) -> usize;
}

/// Sector-oriented storage (common for NOR flash)
pub trait SectorStorage {
    fn sector_size(&self) -> usize;
    fn sector_count(&self) -> usize;
}

/// Async versions of block/sector traits
#[cfg(feature = "async")]
pub trait AsyncBlockStorage: AsyncStorage + BlockStorage {
    async fn read_block(
        &mut self,
        block: usize,
        buf: &mut [u8],
    ) -> Result<(), <Self as AsyncReadStorage>::Error>;
    async fn write_block(
        &mut self,
        block: usize,
        buf: &[u8],
    ) -> Result<(), <Self as AsyncReadStorage>::Error>;
}

#[cfg(feature = "async")]
pub trait AsyncSectorStorage: AsyncStorage + SectorStorage {
    async fn read_sector(
        &mut self,
        sector: usize,
        buf: &mut [u8],
    ) -> Result<(), <Self as AsyncReadStorage>::Error>;
    async fn write_sector(
        &mut self,
        sector: usize,
        buf: &[u8],
    ) -> Result<(), <Self as AsyncReadStorage>::Error>;
}

#[cfg(test)]
mod tests;
