//! # Network abstraction layer for embedded systems
//!
//! This module provides a comprehensive set of traits and implementations for working with
//! different types of network devices in embedded systems. It includes traits for both
//! synchronous and asynchronous operations, as well as support for various network protocols.
//!
//! ## Design Philosophy
//!
//! The network layer is designed around several core principles:
//!
//! - **Protocol Agnostic**: Core traits work with any underlying transport
//! - **Zero-Cost Abstractions**: Traits compile down to direct function calls
//! - **Embedded-First**: Designed for `no_std` environments with limited resources
//! - **Composable**: Mix and match different connection types and protocols
//!
//! ## Architecture
//!
//! The network layer is organized into several abstraction levels:
//!
//! 1. **Core Traits** (`Read`, `Write`, `Close`, `Connection`)
//! 2. **Connection Management** (`Connect`, `Bind`)
//! 3. **Protocol-Specific Extensions** (`Http`, `Mqtt`, `WebSocket`, etc.)
//! 4. **Application Layer** (protocol implementations)
//!
//! ## Usage Examples
//!
//! ### Basic Connection Usage
//!
//! ```rust,no_run
//! use libiot::network::{Connection, Read, Write};
//! # struct MockConnection;
//! # impl Connection for MockConnection {}
//! # impl libiot::network::Read for MockConnection {
//! #     type Error = ();
//! #     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> { Ok(0) }
//! # }
//! # impl libiot::network::Write for MockConnection {
//! #     type Error = ();
//! #     fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> { Ok(0) }
//! #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
//! # }
//! # impl libiot::network::Close for MockConnection {
//! #     type Error = ();
//! #     fn close(self) -> Result<(), Self::Error> { Ok(()) }
//! # }
//!
//! fn communicate_with_device<C: Connection>(mut conn: C) -> Result<(), C::Error> {
//!     let data = b"Hello, device!";
//!     conn.write(data)?;
//!     conn.flush()?;
//!     
//!     let mut response = [0u8; 64];
//!     let bytes_read = conn.read(&mut response)?;
//!     
//!     // Process response...
//!     Ok(())
//! }
//! ```

#![allow(missing_docs)]
#![allow(async_fn_in_trait)]
#![deny(unsafe_code)]

/// Common error types for network operations
pub mod error;

/// OSI Layer 7: Application layer protocol implementations
pub mod application;

/// OSI Layer 4: Transport layer implementations  
pub mod transport;

/// Re-exports of common traits for convenient importing
pub mod prelude {
    #[cfg(feature = "async")]
    pub use super::{AsyncBind, AsyncClose, AsyncConnect, AsyncRead, AsyncWrite};
    pub use super::{Bind, Close, Connect, Read, Write};
}

// ========================
// Core Synchronous Traits
// ========================

/// Trait for reading data from a network connection.
///
/// This trait provides a synchronous interface for reading data from any
/// network connection. It's designed to be simple and efficient for embedded systems.
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::network::Read;
/// # struct MockConnection;
/// # impl Read for MockConnection {
/// #     type Error = std::io::Error;
/// #     fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
/// #         // Mock implementation
/// #         Ok(0)
/// #     }
/// # }
///
/// fn read_data<R: Read>(reader: &mut R) -> Result<Vec<u8>, R::Error> {
///     let mut buffer = [0u8; 1024];
///     let bytes_read = reader.read(&mut buffer)?;
///     Ok(buffer[..bytes_read].to_vec())
/// }
/// ```
pub trait Read {
    /// Associated error type for read operations
    type Error: core::fmt::Debug;

    /// Read data from the connection into the provided buffer.
    ///
    /// Returns the number of bytes read. A return value of 0 typically
    /// indicates that the connection has been closed by the remote end.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to read data into
    ///
    /// # Returns
    ///
    /// * `Ok(n)` - Number of bytes read
    /// * `Err(e)` - Read error occurred
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

/// Trait for writing data to a network connection.
///
/// This trait provides a synchronous interface for writing data to any
/// network connection with support for flushing buffered data.
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::network::Write;
/// # struct MockConnection;
/// # impl Write for MockConnection {
/// #     type Error = std::io::Error;
/// #     fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
/// #         Ok(buf.len())
/// #     }
/// #     fn flush(&mut self) -> Result<(), Self::Error> {
/// #         Ok(())
/// #     }
/// # }
///
/// fn send_message<W: Write>(writer: &mut W, message: &[u8]) -> Result<(), W::Error> {
///     writer.write(message)?;
///     writer.flush()?;
///     Ok(())
/// }
/// ```
pub trait Write {
    /// Associated error type for write operations
    type Error: core::fmt::Debug;

    /// Write data to the connection.
    ///
    /// Returns the number of bytes written. The implementation may write
    /// fewer bytes than requested.
    ///
    /// # Arguments
    ///
    /// * `buf` - Data to write
    ///
    /// # Returns
    ///
    /// * `Ok(n)` - Number of bytes written
    /// * `Err(e)` - Write error occurred
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Flush any buffered write data.
    ///
    /// This ensures that all buffered data is sent over the connection.
    /// Some implementations may be no-ops if no buffering is used.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Flush completed successfully
    /// * `Err(e)` - Flush error occurred
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// Trait for closing network connections.
///
/// Provides a clean way to close connections and free associated resources.
pub trait Close {
    /// Associated error type for close operations
    type Error: core::fmt::Debug;

    /// Close the connection and free any associated resources.
    ///
    /// After calling this method, the connection should not be used for
    /// further operations.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Connection closed successfully
    /// * `Err(e)` - Error occurred while closing
    fn close(self) -> Result<(), Self::Error>;
}

/// A complete synchronous network connection.
///
/// This trait combines reading, writing, and closing capabilities into a
/// single unified interface. Any type implementing this trait can be used
/// with protocol implementations.
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::network::Connection;
///
/// fn use_connection<C: Connection>(mut conn: C) {
///     // Read, write, and eventually close the connection
///     let mut buf = [0u8; 64];
///     if let Ok(n) = conn.read(&mut buf) {
///         // Process received data
///     }
///     
///     if conn.write(b"response").is_ok() {
///         let _ = conn.flush();
///     }
///     
///     // Connection will be closed when dropped
/// }
/// ```
pub trait Connection: Read + Write + Close {}

/// Trait for establishing outbound network connections (client-side).
///
/// This trait is implemented by connection types that can establish
/// connections to remote endpoints.
pub trait Connect {
    /// The type of connection that will be created
    type Connection: Connection;
    /// Associated error type for connection operations
    type Error: core::fmt::Debug;

    /// Establish a connection to a remote endpoint.
    ///
    /// # Arguments
    ///
    /// * `remote` - Address or identifier of the remote endpoint
    ///
    /// # Returns
    ///
    /// * `Ok(connection)` - Connection established successfully
    /// * `Err(e)` - Failed to establish connection
    fn connect(&mut self, remote: &str) -> Result<Self::Connection, Self::Error>;
}

/// Trait for accepting inbound network connections (server-side).
///
/// This trait is implemented by connection types that can listen for
/// and accept incoming connections.
pub trait Bind {
    /// The type of connection that will be created for each client
    type Connection: Connection;
    /// Associated error type for bind operations
    type Error: core::fmt::Debug;

    /// Bind to a local address and accept incoming connections.
    ///
    /// This is typically a blocking operation that waits for a client
    /// to connect.
    ///
    /// # Arguments
    ///
    /// * `local` - Local address to bind to
    ///
    /// # Returns
    ///
    /// * `Ok(connection)` - Incoming connection accepted
    /// * `Err(e)` - Failed to bind or accept connection
    fn bind(&mut self, local: &str) -> Result<Self::Connection, Self::Error>;
}

// ==========================
// Core Asynchronous Traits
// ==========================

/// Trait for reading data from a network connection asynchronously.
///
/// This is the async equivalent of the [`Read`] trait, designed for
/// non-blocking I/O operations in async contexts.
#[cfg(feature = "async")]
pub trait AsyncRead {
    /// Associated error type for async read operations
    type Error: core::fmt::Debug;

    /// Read data from the connection asynchronously.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to read data into
    ///
    /// # Returns
    ///
    /// * `Ok(n)` - Number of bytes read
    /// * `Err(e)` - Read error occurred
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

/// Trait for writing data to a network connection asynchronously.
///
/// This is the async equivalent of the [`Write`] trait, designed for
/// non-blocking I/O operations in async contexts.
#[cfg(feature = "async")]
pub trait AsyncWrite {
    /// Associated error type for async write operations
    type Error: core::fmt::Debug;

    /// Write data to the connection asynchronously.
    ///
    /// # Arguments
    ///
    /// * `buf` - Data to write
    ///
    /// # Returns
    ///
    /// * `Ok(n)` - Number of bytes written
    /// * `Err(e)` - Write error occurred
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Flush any buffered write data asynchronously.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Flush completed successfully
    /// * `Err(e)` - Flush error occurred
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

/// Trait for closing network connections asynchronously.
#[cfg(feature = "async")]
pub trait AsyncClose {
    /// Associated error type for async close operations
    type Error: core::fmt::Debug;

    /// Close the connection asynchronously.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Connection closed successfully
    /// * `Err(e)` - Error occurred while closing
    async fn close(self) -> Result<(), Self::Error>;
}

/// A complete asynchronous network connection.
///
/// This trait combines async reading, writing, and closing capabilities.
#[cfg(feature = "async")]
pub trait AsyncConnection: AsyncRead + AsyncWrite + AsyncClose {}

/// Trait for establishing outbound network connections asynchronously.
#[cfg(feature = "async")]
pub trait AsyncConnect {
    /// The type of connection that will be created
    type Connection: AsyncConnection;
    /// Associated error type for async connection operations
    type Error: core::fmt::Debug;

    /// Establish a connection to a remote endpoint asynchronously.
    ///
    /// # Arguments
    ///
    /// * `remote` - Address or identifier of the remote endpoint
    ///
    /// # Returns
    ///
    /// * `Ok(connection)` - Connection established successfully
    /// * `Err(e)` - Failed to establish connection
    async fn connect(&mut self, remote: &str) -> Result<Self::Connection, Self::Error>;
}

/// Trait for accepting inbound network connections asynchronously.
#[cfg(feature = "async")]
pub trait AsyncBind {
    /// The type of connection that will be created for each client
    type Connection: AsyncConnection;
    /// Associated error type for async bind operations
    type Error: core::fmt::Debug;

    /// Bind to a local address and accept incoming connections asynchronously.
    ///
    /// # Arguments
    ///
    /// * `local` - Local address to bind to
    ///
    /// # Returns
    ///
    /// * `Ok(connection)` - Incoming connection accepted
    /// * `Err(e)` - Failed to bind or accept connection
    async fn bind(&mut self, local: &str) -> Result<Self::Connection, Self::Error>;
}

// ======================
// Protocol-Specific Extensions
// ======================

/// Marker trait for TCP connections.
///
/// This trait indicates that a connection uses the TCP protocol,
/// providing reliability guarantees and ordered delivery.
pub trait Tcp: Connection {}

/// Marker trait for asynchronous TCP connections.
#[cfg(feature = "async")]
pub trait AsyncTcp: AsyncConnection {}

/// Trait for UDP socket operations.
///
/// UDP is a connectionless protocol, so it uses a different interface
/// than connection-based protocols.
pub trait UdpSocket {
    /// Associated error type for UDP operations
    type Error: core::fmt::Debug;

    /// Send data to a specific remote endpoint.
    ///
    /// # Arguments
    ///
    /// * `remote` - Address of the remote endpoint
    /// * `buf` - Data to send
    ///
    /// # Returns
    ///
    /// * `Ok(n)` - Number of bytes sent
    /// * `Err(e)` - Send error occurred
    fn send_to(&mut self, remote: &str, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Receive data from any remote endpoint.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to receive data into
    ///
    /// # Returns
    ///
    /// * `Ok((n, addr))` - Number of bytes received and sender address
    /// * `Err(e)` - Receive error occurred
    fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, &str), Self::Error>;
}

/// Trait for asynchronous UDP socket operations.
#[cfg(feature = "async")]
pub trait AsyncUdpSocket {
    /// Associated error type for async UDP operations
    type Error: core::fmt::Debug;

    /// Send data to a specific remote endpoint asynchronously.
    async fn send_to(&mut self, remote: &str, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Receive data from any remote endpoint asynchronously.
    async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, &str), Self::Error>;
}

/// Marker trait for HTTP connections.
///
/// Indicates that a connection is suitable for HTTP protocol operations.
pub trait Http: Connection {}

/// Marker trait for asynchronous HTTP connections.
#[cfg(feature = "async")]
pub trait AsyncHttp: AsyncConnection {}

/// Marker trait for WebSocket connections.
///
/// Indicates that a connection is suitable for WebSocket protocol operations.
pub trait WebSocket: Connection {}

/// Marker trait for asynchronous WebSocket connections.
#[cfg(feature = "async")]
pub trait AsyncWebSocket: AsyncConnection {}

/// Marker trait for CoAP connections.
///
/// CoAP (Constrained Application Protocol) is designed for resource-constrained
/// devices and networks. While often used over UDP, this trait models it as
/// connection-based for consistency.
pub trait Coap: Connection {}

/// Marker trait for asynchronous CoAP connections.
#[cfg(feature = "async")]
pub trait AsyncCoap: AsyncConnection {}

/// Marker trait for MQTT connections.
///
/// Indicates that a connection is suitable for MQTT protocol operations.
pub trait Mqtt: Connection {}

/// Marker trait for asynchronous MQTT connections.
#[cfg(feature = "async")]
pub trait AsyncMqtt: AsyncConnection {}

/// Marker trait for QUIC connections.
///
/// QUIC is a modern transport protocol that provides many of TCP's reliability
/// guarantees while reducing latency.
pub trait Quic: Connection {}

/// Marker trait for asynchronous QUIC connections.
#[cfg(feature = "async")]
pub trait AsyncQuic: AsyncConnection {}

/// Marker trait for MCP (Model Context Protocol) connections.
///
/// MCP is designed for AI/LLM integration, allowing embedded devices to
/// interact with language models and AI services.
pub trait Mcp: Connection {}

/// Marker trait for asynchronous MCP connections.
#[cfg(feature = "async")]
pub trait AsyncMcp: AsyncConnection {}
