//! # Storage abstraction layer for embedded systems
//!
//! This module provides a comprehensive set of traits and implementations for working with
//! different types of storage devices in embedded systems. It includes traits for both
//! synchronous and asynchronous operations, as well as support for various storage technologies.
//!
//! # Design Philosophy
//!
//! The storage layer is designed around several core principles:
//!
//! - **Technology Agnostic**: Core traits work with any storage technology
//! - **Zero-Cost Abstractions**: Traits compile down to direct hardware calls
//! - **Embedded-First**: Designed for `no_std` environments with limited resources
//! - **Safety**: Strong typing prevents common storage access errors
//! - **Composable**: Mix and match different storage types and interfaces
//!
//! # Architecture Overview
//!
//! The storage layer is organized into several abstraction levels:
//!
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   Application   │    │   File System   │    │     Cache       │
//! │     Layer       │    │     Layer       │    │     Layer       │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//!           │                        │                        │
//!           ▼                        ▼                        ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Storage Abstraction Layer                    │
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────┐  │
//! │  │  Core Traits  │  │  Block/Sector │  │  Technology       │  │
//! │  │               │  │  Management   │  │  Specific Traits  │  │
//! │  └───────────────┘  └───────────────┘  └───────────────────┘  │
//! └─────────────────────────────────────────────────────────────────┘
//!           │                        │                        │
//!           ▼                        ▼                        ▼
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   Flash Memory  │    │   EEPROM/FRAM   │    │   SD/MMC Cards  │
//! │   (NOR/NAND)    │    │                 │    │                 │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//! ```
//!
//! # Core Traits
//!
//! ## Basic Storage Operations
//!
//! - [`ReadStorage`]: Read data from storage
//! - [`Storage`]: Read and write operations
//! - [`BlockingErase`]: Synchronous erase operations
//! - [`AsyncStorage`]: Asynchronous storage operations (with `async` feature)
//!
//! ## Organization Traits
//!
//! - [`BlockStorage`]: Block-oriented storage (SD cards, NAND flash)
//! - [`SectorStorage`]: Sector-oriented storage (NOR flash)
//! - [`Region`]: Memory region management
//!
//! ## Technology-Specific Traits
//!
//! - [`Eeprom`]: EEPROM-specific operations
//! - [`NandFlash`]: NAND flash with spare area support
//! - [`SdMmc`]: SD/MMC card operations
//! - [`Fram`]: Ferroelectric RAM operations
//! - [`RamStorage`]: RAM-based storage
//!
//! # Usage Examples
//!
//! ## Basic Storage Operations
//!
//! ```rust,no_run
//! use libiot::storage::{ReadStorage, Storage};
//!
//! fn read_sensor_data<S: ReadStorage>(storage: &mut S) -> Result<[u8; 4], S::Error> {
//!     let mut data = [0u8; 4];
//!     storage.read(0x1000, &mut data)?;
//!     Ok(data)
//! }
//!
//! fn store_configuration<S: Storage>(storage: &mut S, config: &[u8]) -> Result<(), S::Error> {
//!     storage.write(0x2000, config)?;
//!     Ok(())
//! }
//! ```
//!
//! ## Block-Based Storage
//!
//! ```rust,no_run
//! use libiot::storage::{BlockStorage, Storage};
//!
//! fn read_block<S: Storage + BlockStorage>(storage: &mut S, block_num: usize) -> Result<Vec<u8>, S::Error> {
//!     let block_size = storage.block_size();
//!     let mut buffer = vec![0u8; block_size];
//!     let offset = (block_num * block_size) as u32;
//!     storage.read(offset, &mut buffer)?;
//!     Ok(buffer)
//! }
//! ```
//!
//! ## Flash Memory with Erase
//!
//! ```rust,no_run
//! use libiot::storage::{Storage, BlockingErase};
//!
//! fn update_firmware_block<S: Storage + BlockingErase>(
//!     storage: &mut S,
//!     start_addr: u32,
//!     end_addr: u32,
//!     new_data: &[u8]
//! ) -> Result<(), S::Error> {
//!     // Erase the region first
//!     storage.erase(start_addr, end_addr)?;
//!     // Write new data
//!     storage.write(start_addr, new_data)?;
//!     Ok(())
//! }
//! ```

#![allow(missing_docs)]
#![allow(async_fn_in_trait)]
#![deny(unsafe_code)]

/// Common error types for storage operations
pub mod error;

/// Re-exports of common traits for convenient importing
pub mod prelude {
    #[cfg(feature = "async")]
    pub use super::{
        AsyncBlockStorage, AsyncErase, AsyncReadStorage, AsyncSectorStorage, AsyncStorage,
    };
    pub use super::{BlockStorage, BlockingErase, ReadStorage, Region, SectorStorage, Storage};
}

/// A contiguous memory region with start and end boundaries.
///
/// This trait provides a standardized way to represent memory regions,
/// which is useful for defining valid address ranges, protected areas,
/// or organizing storage into logical partitions.
///
/// # Examples
///
/// ```rust
/// use libiot::storage::Region;
///
/// struct FlashRegion {
///     start: u32,
///     size: u32,
/// }
///
/// impl Region for FlashRegion {
///     fn start(&self) -> u32 {
///         self.start
///     }
///
///     fn end(&self) -> u32 {
///         self.start + self.size
///     }
/// }
///
/// let bootloader = FlashRegion { start: 0x0000, size: 0x4000 };
/// let application = FlashRegion { start: 0x4000, size: 0x1C000 };
///
/// assert!(bootloader.contains(0x2000));
/// assert!(!bootloader.contains(0x5000));
/// assert!(application.contains(0x5000));
/// ```
pub trait Region {
    /// Start address of the region (inclusive).
    ///
    /// This is the lowest valid address within the region.
    fn start(&self) -> u32;

    /// End address of the region (exclusive).
    ///
    /// This is one past the highest valid address within the region.
    /// The actual valid range is `start()..end()`.
    fn end(&self) -> u32;

    /// Check if an address is contained within this region.
    ///
    /// Returns `true` if the address is within the valid range
    /// `[start(), end())`, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `address` - The address to check
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::storage::Region;
    /// # struct TestRegion;
    /// # impl Region for TestRegion {
    /// #     fn start(&self) -> u32 { 0x1000 }
    /// #     fn end(&self) -> u32 { 0x2000 }
    /// # }
    ///
    /// let region = TestRegion;
    /// assert!(region.contains(0x1000));  // At start
    /// assert!(region.contains(0x1500));  // In middle
    /// assert!(!region.contains(0x2000)); // At end (exclusive)
    /// assert!(!region.contains(0x0500)); // Before start
    /// ```
    fn contains(&self, address: u32) -> bool {
        (address >= self.start()) && (address < self.end())
    }
}

// ========================
// Core Synchronous Traits
// ========================

/// Trait for reading data from storage devices.
///
/// This is the fundamental trait for all readable storage devices. It provides
/// a simple interface for reading data at specific offsets without requiring
/// write capabilities.
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::storage::ReadStorage;
///
/// fn read_device_id<S: ReadStorage>(storage: &mut S) -> Result<u32, S::Error> {
///     let mut id_bytes = [0u8; 4];
///     storage.read(0, &mut id_bytes)?;
///     Ok(u32::from_le_bytes(id_bytes))
/// }
/// ```
pub trait ReadStorage {
    /// Associated error type for read operations
    type Error: core::fmt::Debug;

    /// Read data from the storage device.
    ///
    /// Reads data from the specified offset into the provided buffer.
    /// The entire buffer will be filled unless an error occurs.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset from the start of the storage device
    /// * `bytes` - Buffer to read data into
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data read successfully
    /// * `Err(error)` - Read operation failed
    ///
    /// # Errors
    ///
    /// - `OutOfBounds` if offset + buffer length exceeds device capacity
    /// - `ReadError` if hardware read operation fails
    /// - `NotInitialized` if device is not properly initialized
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;

    /// Get the total capacity of the storage device in bytes.
    ///
    /// This returns the maximum number of bytes that can be stored
    /// on the device, which determines the valid range for read operations.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libiot::storage::ReadStorage;
    /// # struct MockStorage;
    /// # impl ReadStorage for MockStorage {
    /// #     type Error = ();
    /// #     fn read(&mut self, _offset: u32, _bytes: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn capacity(&self) -> usize { 1024 * 1024 }
    /// # }
    ///
    /// let storage = MockStorage;
    /// println!("Storage capacity: {} bytes", storage.capacity());
    /// ```
    fn capacity(&self) -> usize;
}

/// Trait for storage devices that support both read and write operations.
///
/// This trait extends [`ReadStorage`] to include write capabilities,
/// making it suitable for general-purpose storage operations.
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::storage::Storage;
///
/// fn save_config<S: Storage>(storage: &mut S, config: &[u8]) -> Result<(), S::Error> {
///     // Write configuration to a known location
///     storage.write(0x1000, config)?;
///     
///     // Verify by reading it back
///     let mut verify_buf = vec![0u8; config.len()];
///     storage.read(0x1000, &mut verify_buf)?;
///     
///     if verify_buf == config {
///         Ok(())
///     } else {
///         // Would need custom error type for this
///         Ok(()) // Simplified for example
///     }
/// }
/// ```
pub trait Storage: ReadStorage {
    /// Write data to the storage device.
    ///
    /// Writes the provided data to the specified offset. The behavior
    /// when writing to already-written locations depends on the storage
    /// technology (some require erase, others support overwrites).
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset from the start of the storage device
    /// * `bytes` - Data to write to the device
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data written successfully
    /// * `Err(error)` - Write operation failed
    ///
    /// # Errors
    ///
    /// - `OutOfBounds` if offset + data length exceeds device capacity
    /// - `WriteError` if hardware write operation fails
    /// - `NotInitialized` if device is not properly initialized
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libiot::storage::Storage;
    /// # struct MockStorage;
    /// # impl libiot::storage::ReadStorage for MockStorage {
    /// #     type Error = ();
    /// #     fn read(&mut self, _offset: u32, _bytes: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn capacity(&self) -> usize { 1024 }
    /// # }
    /// # impl Storage for MockStorage {
    /// #     fn write(&mut self, _offset: u32, _bytes: &[u8]) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    ///
    /// let mut storage = MockStorage;
    /// let data = b"Hello, Storage!";
    /// storage.write(0, data).unwrap();
    /// ```
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;
}

/// Trait for storage devices that support erase operations.
///
/// Many storage technologies (especially flash memory) require explicit
/// erase operations before writing new data. This trait provides a
/// synchronous interface for erasing storage regions.
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::storage::{Storage, BlockingErase};
///
/// fn clear_log_area<S: Storage + BlockingErase>(storage: &mut S) -> Result<(), S::Error> {
///     let log_start = 0x10000;
///     let log_end = 0x20000;
///     
///     // Erase the entire log area
///     storage.erase(log_start, log_end)?;
///     
///     // Now we can write new log entries
///     storage.write(log_start, b"Log cleared\n")?;
///     Ok(())
/// }
/// ```
pub trait BlockingErase: Storage {
    /// Erase a region of storage.
    ///
    /// Erases all data in the specified address range. After erasing,
    /// the erased region should read as `0xFF` bytes (flash memory convention).
    /// The exact behavior depends on the storage technology.
    ///
    /// # Arguments
    ///
    /// * `from` - Start address of the region to erase (inclusive)
    /// * `to` - End address of the region to erase (exclusive)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Erase completed successfully
    /// * `Err(error)` - Erase operation failed
    ///
    /// # Errors
    ///
    /// - `OutOfBounds` if the address range is invalid
    /// - `EraseError` if the hardware erase operation fails
    /// - `StorageFault` if the storage area is damaged and cannot be erased
    ///
    /// # Note
    ///
    /// Some storage devices have alignment requirements for erase operations
    /// (e.g., must erase entire blocks). Check device documentation for
    /// specific requirements.
    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error>;
}

// ========================
// Core Asynchronous Traits
// ========================

/// Trait for reading data from storage devices asynchronously.
///
/// This is the async equivalent of [`ReadStorage`], designed for
/// non-blocking storage operations in async contexts.
#[cfg(feature = "async")]
pub trait AsyncReadStorage {
    /// Associated error type for async read operations
    type Error: core::fmt::Debug;

    /// Read data from the storage device asynchronously.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset from the start of the storage device
    /// * `bytes` - Buffer to read data into
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data read successfully
    /// * `Err(error)` - Read operation failed
    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;

    /// Get the total capacity of the storage device in bytes.
    fn capacity(&self) -> usize;
}

/// Trait for storage devices that support both read and write operations asynchronously.
#[cfg(feature = "async")]
pub trait AsyncStorage: AsyncReadStorage {
    /// Write data to the storage device asynchronously.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset from the start of the storage device
    /// * `bytes` - Data to write to the device
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data written successfully
    /// * `Err(error)` - Write operation failed
    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;
}

/// Trait for storage devices that support erase operations asynchronously.
#[cfg(feature = "async")]
pub trait AsyncErase: AsyncStorage {
    /// Erase a region of storage asynchronously.
    ///
    /// # Arguments
    ///
    /// * `from` - Start address of the region to erase (inclusive)
    /// * `to` - End address of the region to erase (exclusive)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Erase completed successfully
    /// * `Err(error)` - Erase operation failed
    async fn erase(&mut self, from: u32, to: u32) -> Result<(), <Self as AsyncReadStorage>::Error>;
}

// ======================
// Technology-Specific Extensions
// ======================

/// EEPROM-specific storage operations.
///
/// EEPROM (Electrically Erasable Programmable Read-Only Memory) has specific
/// characteristics that benefit from specialized operations, such as byte-level
/// write verification and page-based programming.
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::storage::{Storage, Eeprom};
///
/// fn store_critical_byte<E: Storage + Eeprom>(eeprom: &mut E, addr: u32, value: u8) -> Result<(), E::Error> {
///     // Use verified write for critical data
///     eeprom.write_verified(addr, value)?;
///     Ok(())
/// }
/// ```
pub trait Eeprom: Storage {
    /// Write a single byte with automatic verification.
    ///
    /// This operation writes a byte and then reads it back to verify
    /// the write was successful. This is important for EEPROM devices
    /// which can have write failures due to wear or power issues.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset to write to
    /// * `byte` - Byte value to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Byte written and verified successfully
    /// * `Err(error)` - Write or verification failed
    fn write_verified(&mut self, offset: u32, byte: u8)
    -> Result<(), <Self as ReadStorage>::Error>;

    /// Get the page size for page-mode programming.
    ///
    /// Many EEPROM devices support page-mode programming which is more
    /// efficient than byte-by-byte writes. Returns `None` if the device
    /// doesn't support page programming.
    ///
    /// # Returns
    ///
    /// * `Some(size)` - Page size in bytes
    /// * `None` - Device doesn't support page programming
    fn page_size(&self) -> Option<usize>;
}

/// SD/MMC card specific operations.
///
/// SD and MMC cards require special initialization sequences and have
/// specific status information that can be useful for applications.
pub trait SdMmc: Storage + BlockStorage + SectorStorage {
    /// Initialize the SD/MMC card.
    ///
    /// Performs the required initialization sequence for the card,
    /// including power-up, command initialization, and capacity detection.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Card initialized successfully
    /// * `Err(error)` - Initialization failed
    fn init(&mut self) -> Result<(), <Self as ReadStorage>::Error>;

    /// Get the current card status.
    ///
    /// Returns detailed status information about the card including
    /// capacity, protection status, and other card-specific information.
    ///
    /// # Returns
    ///
    /// * `Ok(status)` - Current card status
    /// * `Err(error)` - Failed to read status
    fn status(&mut self) -> Result<SdMmcStatus, <Self as ReadStorage>::Error>;

    /// Set the block length for SDSC cards.
    ///
    /// Standard Capacity (SDSC) cards support configurable block lengths.
    /// High Capacity (SDHC/SDXC) cards always use 512-byte blocks.
    ///
    /// # Arguments
    ///
    /// * `len` - Block length in bytes (typically 512)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Block length set successfully
    /// * `Err(error)` - Failed to set block length
    fn set_block_length(&mut self, len: u32) -> Result<(), <Self as ReadStorage>::Error>;
}

/// SD/MMC card status information.
///
/// Contains important status and capability information about an SD/MMC card.
pub struct SdMmcStatus {
    /// Whether the card has been successfully initialized.
    pub initialized: bool,
    /// Total card capacity in bytes.
    pub capacity: u64,
    /// Whether the card has write protection enabled.
    pub protection: bool,
    // Additional fields can be added as needed
}

/// RAM-based storage operations.
///
/// RAM storage provides the fastest access but is volatile. This trait
/// provides operations specific to RAM-based storage including direct
/// memory access.
pub trait RamStorage: Storage {
    /// Clear the entire storage area.
    ///
    /// Sets all bytes in the storage to zero. This is only meaningful
    /// for volatile storage as non-volatile storage would lose this
    /// state on power loss.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Storage cleared successfully
    /// * `Err(error)` - Clear operation failed
    fn clear(&mut self) -> Result<(), <Self as ReadStorage>::Error>;

    /// Get a read-only reference to the underlying memory.
    ///
    /// Provides direct access to the storage memory for efficient
    /// read operations without copying data.
    ///
    /// # Returns
    ///
    /// A slice representing the entire storage contents
    fn as_slice(&self) -> &[u8];

    /// Get a mutable reference to the underlying memory.
    ///
    /// Provides direct access to the storage memory for efficient
    /// read-write operations without copying data.
    ///
    /// # Returns
    ///
    /// A mutable slice representing the entire storage contents
    fn as_mut_slice(&mut self) -> &mut [u8];
}

/// NAND Flash specific operations.
///
/// NAND Flash memory has unique characteristics including spare areas
/// for metadata and bad block management that require specialized operations.
pub trait NandFlash: Storage + BlockStorage {
    /// Read spare area data from a specific page.
    ///
    /// NAND Flash pages have main data areas and spare areas for metadata
    /// such as error correction codes and bad block markers.
    ///
    /// # Arguments
    ///
    /// * `block` - Block number
    /// * `page` - Page number within the block
    /// * `spare` - Buffer to read spare data into
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Spare data read successfully
    /// * `Err(error)` - Read operation failed
    fn read_spare(
        &mut self,
        block: usize,
        page: usize,
        spare: &mut [u8],
    ) -> Result<(), <Self as ReadStorage>::Error>;

    /// Write data with spare area.
    ///
    /// Writes both main data and spare area data to a page. This is
    /// commonly used to write error correction codes along with the data.
    ///
    /// # Arguments
    ///
    /// * `block` - Block number
    /// * `page` - Page number within the block
    /// * `data` - Main data to write
    /// * `spare` - Spare area data to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data written successfully
    /// * `Err(error)` - Write operation failed
    fn write_with_spare(
        &mut self,
        block: usize,
        page: usize,
        data: &[u8],
        spare: &[u8],
    ) -> Result<(), <Self as ReadStorage>::Error>;

    /// Check if a block is good or bad.
    ///
    /// NAND Flash can develop bad blocks over time. This function
    /// checks the block status to determine if it's safe to use.
    ///
    /// # Arguments
    ///
    /// * `block` - Block number to check
    ///
    /// # Returns
    ///
    /// * `Ok(status)` - Block status
    /// * `Err(error)` - Failed to check block status
    fn block_status(&mut self, block: usize) -> Result<BlockStatus, <Self as ReadStorage>::Error>;
}

/// NAND Flash block status.
///
/// Indicates whether a block is usable or should be avoided.
pub enum BlockStatus {
    /// Block is good and can be used normally.
    Good,
    /// Block is bad and should not be used.
    Bad,
    /// Block is reserved for special purposes.
    Reserved,
}

/// FRAM (Ferroelectric RAM) operations.
///
/// FRAM combines the benefits of RAM (fast access) with non-volatility.
/// It has unique characteristics such as virtually unlimited endurance
/// and no erase requirement.
pub trait Fram: Storage {
    /// Get the write endurance specification.
    ///
    /// FRAM typically has much higher endurance than flash memory.
    /// This returns the manufacturer's specification for write cycles.
    ///
    /// # Returns
    ///
    /// * `Some(cycles)` - Maximum write cycles supported
    /// * `None` - Endurance specification not available
    fn endurance(&self) -> Option<u32>;

    /// Check if the storage technology requires erase operations.
    ///
    /// FRAM can be written directly without erasing, unlike flash memory.
    /// This method returns `false` for FRAM implementations.
    ///
    /// # Returns
    ///
    /// `false` for FRAM as it doesn't require erase operations
    fn requires_erase(&self) -> bool {
        false
    }
}

// ======================
// Composite Traits
// ======================

/// Unified storage interface for both volatile and non-volatile storage.
///
/// This trait allows code to work with different storage types transparently
/// while still being able to determine the persistence characteristics.
pub trait UnifiedStorage: Storage {
    /// Check if the storage retains data without power.
    ///
    /// # Returns
    ///
    /// * `true` - Storage is non-volatile (retains data without power)
    /// * `false` - Storage is volatile (loses data when power is removed)
    fn is_non_volatile(&self) -> bool;
}

/// Block-oriented storage interface.
///
/// Many storage devices organize data into fixed-size blocks. This trait
/// provides information about the block structure for devices that use
/// block-based organization.
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::storage::{Storage, BlockStorage};
///
/// fn read_entire_block<S: Storage + BlockStorage>(
///     storage: &mut S,
///     block_num: usize
/// ) -> Result<Vec<u8>, S::Error> {
///     let block_size = storage.block_size();
///     let mut buffer = vec![0u8; block_size];
///     let offset = (block_num * block_size) as u32;
///     storage.read(offset, &mut buffer)?;
///     Ok(buffer)
/// }
/// ```
pub trait BlockStorage {
    /// Get the size of each block in bytes.
    ///
    /// # Returns
    ///
    /// Block size in bytes (commonly 512, 4096, or 8192)
    fn block_size(&self) -> usize;

    /// Get the total number of blocks on the device.
    ///
    /// # Returns
    ///
    /// Total number of blocks available
    fn block_count(&self) -> usize;
}

/// Sector-oriented storage interface.
///
/// Some storage devices organize data into sectors, which may be different
/// from blocks. This is common in NOR flash and some legacy storage systems.
pub trait SectorStorage {
    /// Get the size of each sector in bytes.
    ///
    /// # Returns
    ///
    /// Sector size in bytes
    fn sector_size(&self) -> usize;

    /// Get the total number of sectors on the device.
    ///
    /// # Returns
    ///
    /// Total number of sectors available
    fn sector_count(&self) -> usize;
}

/// Asynchronous block-oriented storage operations.
#[cfg(feature = "async")]
pub trait AsyncBlockStorage: AsyncStorage + BlockStorage {
    /// Read an entire block asynchronously.
    ///
    /// # Arguments
    ///
    /// * `block` - Block number to read
    /// * `buf` - Buffer to read data into (must be at least block_size() bytes)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Block read successfully
    /// * `Err(error)` - Read operation failed
    async fn read_block(
        &mut self,
        block: usize,
        buf: &mut [u8],
    ) -> Result<(), <Self as AsyncReadStorage>::Error>;

    /// Write an entire block asynchronously.
    ///
    /// # Arguments
    ///
    /// * `block` - Block number to write
    /// * `buf` - Data to write (must be exactly block_size() bytes)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Block written successfully
    /// * `Err(error)` - Write operation failed
    async fn write_block(
        &mut self,
        block: usize,
        buf: &[u8],
    ) -> Result<(), <Self as AsyncReadStorage>::Error>;
}

/// Asynchronous sector-oriented storage operations.
#[cfg(feature = "async")]
pub trait AsyncSectorStorage: AsyncStorage + SectorStorage {
    /// Read an entire sector asynchronously.
    ///
    /// # Arguments
    ///
    /// * `sector` - Sector number to read
    /// * `buf` - Buffer to read data into (must be at least sector_size() bytes)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Sector read successfully
    /// * `Err(error)` - Read operation failed
    async fn read_sector(
        &mut self,
        sector: usize,
        buf: &mut [u8],
    ) -> Result<(), <Self as AsyncReadStorage>::Error>;

    /// Write an entire sector asynchronously.
    ///
    /// # Arguments
    ///
    /// * `sector` - Sector number to write
    /// * `buf` - Data to write (must be exactly sector_size() bytes)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Sector written successfully
    /// * `Err(error)` - Write operation failed
    async fn write_sector(
        &mut self,
        sector: usize,
        buf: &[u8],
    ) -> Result<(), <Self as AsyncReadStorage>::Error>;
}
