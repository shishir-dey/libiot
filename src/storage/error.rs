//! Common error types for storage operations.
//!
//! This module defines error types that are used throughout the storage layer
//! to provide consistent error handling across different storage technologies
//! and device types. The errors are designed to be comprehensive enough for
//! proper error handling while remaining simple for embedded environments.

/// A common error type for storage operations.
///
/// This enum defines a set of common errors that can occur when working with
/// storage devices. It is designed to be simple and portable for `no_std`
/// environments while providing enough granularity for proper error handling
/// across different storage technologies.
///
/// # Usage Examples
///
/// ```rust
/// use libiot::storage::error::Error;
///
/// fn handle_storage_error(error: Error) {
///     match error {
///         Error::OutOfBounds => {
///             println!("Attempted to access invalid address");
///         }
///         Error::WriteError => {
///             println!("Failed to write data to storage");
///         }
///         Error::ReadError => {
///             println!("Failed to read data from storage");
///         }
///         Error::EraseError => {
///             println!("Failed to erase storage block");
///         }
///         Error::NotInitialized => {
///             println!("Storage device not properly initialized");
///         }
///         Error::CardError => {
///             println!("SD/MMC card specific error");
///         }
///         Error::StorageFault => {
///             println!("Hardware fault detected in storage");
///         }
///     }
/// }
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    /// An operation was attempted on an address that is out of bounds.
    ///
    /// This error occurs when trying to access memory addresses beyond the
    /// valid range of the storage device. It can happen with:
    /// - Read/write operations beyond device capacity
    /// - Invalid block or sector numbers
    /// - Misaligned access patterns
    OutOfBounds,

    /// An error occurred during a write operation.
    ///
    /// Write errors can be caused by:
    /// - Hardware failure during write
    /// - Attempting to write to read-only storage
    /// - Power loss during write operation
    /// - Write protection enabled
    /// - Storage device is full or worn out
    WriteError,

    /// An error occurred during a read operation.
    ///
    /// Read errors typically indicate:
    /// - Data corruption due to aging or wear
    /// - Hardware failure in the storage device
    /// - Communication errors with the storage controller
    /// - Power supply issues during read
    ReadError,

    /// An error occurred during an erase operation.
    ///
    /// Erase errors are common with flash memory and can indicate:
    /// - Block is marked as bad and cannot be erased
    /// - Hardware failure during erase cycle
    /// - Erase verification failed
    /// - Device reached end of life for erase cycles
    EraseError,

    /// An operation was attempted on a device that was not initialized.
    ///
    /// This error occurs when:
    /// - Device initialization sequence was not completed
    /// - Required setup operations were skipped
    /// - Device was reset without proper re-initialization
    /// - Communication with device could not be established
    NotInitialized,

    /// A card-specific error occurred (e.g., for SD/MMC cards).
    ///
    /// This error type covers SD and MMC card specific failures:
    /// - Card detection/insertion issues
    /// - Unsupported card type or format
    /// - Card authentication or encryption errors
    /// - Card command sequence errors
    /// - Card protection switch activated
    CardError,

    /// The underlying storage is bad/unusable at a specific location.
    ///
    /// This indicates a hardware fault that makes the storage unreliable:
    /// - Bad blocks in flash memory
    /// - Corrupted sectors that cannot be recovered
    /// - Physical damage to storage medium
    /// - Excessive wear that makes area unusable
    /// - Manufacturing defects discovered during operation
    StorageFault,
}

#[cfg(feature = "defmt")]
impl defmt::Format for Error {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Error::OutOfBounds => defmt::write!(f, "OutOfBounds"),
            Error::WriteError => defmt::write!(f, "WriteError"),
            Error::ReadError => defmt::write!(f, "ReadError"),
            Error::EraseError => defmt::write!(f, "EraseError"),
            Error::NotInitialized => defmt::write!(f, "NotInitialized"),
            Error::CardError => defmt::write!(f, "CardError"),
            Error::StorageFault => defmt::write!(f, "StorageFault"),
        }
    }
}
