//! A network abstraction layer for embedded systems
//!
//! This crate provides a comprehensive set of traits and implementations for working with
//! different types of network devices in embedded systems. It includes traits for both
//! synchronous and asynchronous operations, as well as support for various network protocols.
//!

#![allow(missing_docs)]
#![allow(async_fn_in_trait)]
#![deny(unsafe_code)]

/// Common error types for network operations
pub mod error;

/// Protocol-specific client implementations
pub mod client;

/// Re-exports of common traits
pub mod prelude {
    #[cfg(feature = "async")]
    pub use super::{AsyncBind, AsyncClose, AsyncConnect, AsyncRead, AsyncWrite};
    pub use super::{Bind, Close, Connect, Read, Write};
}

// Core synchronous traits
pub trait Read {
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Read data from the connection
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

pub trait Write {
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Write data to the connection
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
    /// Flush the write buffer
    fn flush(&mut self) -> Result<(), Self::Error>;
}

pub trait Close {
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Close the connection
    fn close(self) -> Result<(), Self::Error>;
}

/// A synchronous connection
pub trait Connection: Read + Write + Close {}

/// A synchronous connector (client)
pub trait Connect {
    /// Associated connection type
    type Connection: Connection;
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Open a connection
    fn connect(&mut self, remote: &str) -> Result<Self::Connection, Self::Error>;
}

/// A synchronous binder (server)
pub trait Bind {
    /// Associated connection type
    type Connection: Connection;
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Bind to a local address and wait for incoming connections
    fn bind(&mut self, local: &str) -> Result<Self::Connection, Self::Error>;
}

// Core async traits
#[cfg(feature = "async")]
pub trait AsyncRead {
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Read data from the connection asynchronously
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

#[cfg(feature = "async")]
pub trait AsyncWrite {
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Write data to the connection asynchronously
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
    /// Flush the write buffer asynchronously
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

#[cfg(feature = "async")]
pub trait AsyncClose {
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Close the connection asynchronously
    async fn close(self) -> Result<(), Self::Error>;
}

#[cfg(feature = "async")]
pub trait AsyncConnection: AsyncRead + AsyncWrite + AsyncClose {}

#[cfg(feature = "async")]
pub trait AsyncConnect {
    /// Associated connection type
    type Connection: AsyncConnection;
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Open a connection asynchronously
    async fn connect(&mut self, remote: &str) -> Result<Self::Connection, Self::Error>;
}

#[cfg(feature = "async")]
pub trait AsyncBind {
    /// Associated connection type
    type Connection: AsyncConnection;
    /// Associated error type
    type Error: core::fmt::Debug;
    /// Bind to a local address and wait for incoming connections asynchronously
    async fn bind(&mut self, local: &str) -> Result<Self::Connection, Self::Error>;
}

/// ======================
/// Protocol-Specific Extensions
/// ======================

// TCP
pub trait Tcp: Connection {}
#[cfg(feature = "async")]
pub trait AsyncTcp: AsyncConnection {}

// UDP
pub trait UdpSocket {
    type Error: core::fmt::Debug;
    fn send_to(&mut self, remote: &str, buf: &[u8]) -> Result<usize, Self::Error>;
    fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, &str), Self::Error>;
}
#[cfg(feature = "async")]
pub trait AsyncUdpSocket {
    type Error: core::fmt::Debug;
    async fn send_to(&mut self, remote: &str, buf: &[u8]) -> Result<usize, Self::Error>;
    async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, &str), Self::Error>;
}

// HTTP
pub trait Http: Connection {}
#[cfg(feature = "async")]
pub trait AsyncHttp: AsyncConnection {}

// WebSocket
pub trait WebSocket: Connection {}
#[cfg(feature = "async")]
pub trait AsyncWebSocket: AsyncConnection {}

// CoAP
pub trait Coap: Connection {} // Often over UDP, but can be over TCP. Modeling as connection-based for now.
#[cfg(feature = "async")]
pub trait AsyncCoap: AsyncConnection {}

// MQTT
pub trait Mqtt: Connection {}
#[cfg(feature = "async")]
pub trait AsyncMqtt: AsyncConnection {}

// QUIC
pub trait Quic: Connection {}
#[cfg(feature = "async")]
pub trait AsyncQuic: AsyncConnection {}

// MCP (Model Context Protocol)
pub trait Mcp: Connection {}
#[cfg(feature = "async")]
pub trait AsyncMcp: AsyncConnection {}
