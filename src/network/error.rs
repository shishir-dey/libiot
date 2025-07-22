//! Common error types for network operations
//!
//! This module defines error types that are used throughout the network layer
//! to provide consistent error handling across different protocols and connection types.

/// A common error type for network operations.
///
/// This enum defines a set of common errors that can occur when working with
/// network devices. It is designed to be simple and portable for `no_std`
/// environments while providing enough detail for proper error handling.
///
/// # Examples
///
/// ```rust
/// use libiot::network::error::Error;
///
/// fn handle_network_error(error: Error) {
///     match error {
///         Error::ConnectionRefused => {
///             println!("Connection was refused by the remote server");
///         }
///         Error::Timeout => {
///             println!("Operation timed out");
///         }
///         Error::WriteError => {
///             println!("Failed to write data to the connection");
///         }
///         _ => {
///             println!("Other network error occurred");
///         }
///     }
/// }
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    /// An operation was attempted on a connection that is not open.
    ///
    /// This error occurs when trying to read from, write to, or perform other
    /// operations on a connection that hasn't been established or has been closed.
    NotOpen,

    /// An error occurred during a write operation.
    ///
    /// This can happen due to network issues, buffer overflow, or the remote
    /// end closing the connection during a write operation.
    WriteError,

    /// An error occurred during a read operation.
    ///
    /// This can happen due to network issues, malformed data, or the remote
    /// end closing the connection during a read operation.
    ReadError,

    /// A connection attempt was refused by the remote server.
    ///
    /// This typically occurs when the remote server is not accepting connections,
    /// the port is closed, or authentication fails.
    ConnectionRefused,

    /// A timeout occurred during a network operation.
    ///
    /// This happens when an operation takes longer than the configured timeout
    /// period, which could indicate network congestion or unresponsive remote endpoints.
    Timeout,

    /// The connection was closed unexpectedly.
    ///
    /// This occurs when the remote endpoint closes the connection, either gracefully
    /// or due to an error condition.
    ConnectionClosed,

    /// An invalid address was provided.
    ///
    /// This error is returned when the provided network address is malformed,
    /// unreachable, or otherwise invalid.
    InvalidAddress,

    /// A protocol-specific error occurred.
    ///
    /// This is a catch-all error for protocol-specific issues such as:
    /// - Malformed protocol messages
    /// - Unsupported protocol versions
    /// - Protocol state violations
    /// - Invalid protocol parameters
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
