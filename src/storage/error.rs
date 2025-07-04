//! Common error types for storage operations

/// A common error type for storage operations.
///
/// This enum defines a set of common errors that can occur when working with
/// storage devices. It is designed to be simple and portable for `no_std`
/// environments.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    /// An operation was attempted on an address that is out of bounds.
    OutOfBounds,
    /// An error occurred during a write operation.
    WriteError,
    /// An error occurred during a read operation.
    ReadError,
    /// An error occurred during an erase operation.
    EraseError,
    /// An operation was attempted on a device that was not initialized.
    NotInitialized,
    /// A card-specific error occurred (e.g., for SD/MMC cards).
    CardError,
    /// The underlying storage is bad/unusable at a specific location
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
