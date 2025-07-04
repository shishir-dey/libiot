//! Common error types for network operations

/// A common error type for network operations.
///
/// This enum defines a set of common errors that can occur when working with
/// network devices. It is designed to be simple and portable for `no_std`
/// environments.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    /// An operation was attempted on a connection that is not open.
    NotOpen,
    /// An error occurred during a write operation.
    WriteError,
    /// An error occurred during a read operation.
    ReadError,
    /// A connection attempt was refused.
    ConnectionRefused,
    /// A timeout occurred.
    Timeout,
    /// The connection was closed.
    ConnectionClosed,
    /// An invalid address was provided.
    InvalidAddress,
    /// A protocol-specific error occurred.
    ProtocolError,
}

#[cfg(feature = "defmt")]
impl defmt::Format for Error {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Error::NotOpen => defmt::write!(f, "NotOpen"),
            Error::WriteError => defmt::write!(f, "WriteError"),
            Error::ReadError => defmt::write!(f, "ReadError"),
            Error::ConnectionRefused => defmt::write!(f, "ConnectionRefused"),
            Error::Timeout => defmt::write!(f, "Timeout"),
            Error::ConnectionClosed => defmt::write!(f, "ConnectionClosed"),
            Error::InvalidAddress => defmt::write!(f, "InvalidAddress"),
            Error::ProtocolError => defmt::write!(f, "ProtocolError"),
        }
    }
}
