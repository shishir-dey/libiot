//! # Application Layer Network Protocols
//!
//! This module contains implementations of various application layer (OSI Layer 7)
//! network protocols commonly used in IoT applications. Each protocol is designed
//! to work with the core network traits and provide a consistent API for embedded systems.
//!
//! ## Available Protocols
//!
//! - **[`http`]**: HTTP/1.1 client implementation for RESTful API communication
//! - **[`mqtt`]**: MQTT 3.1.1 client for lightweight publish-subscribe messaging  
//! - **[`mcp`]**: Model Context Protocol client for AI/LLM integration
//! - **[`websocket`]**: WebSocket protocol for real-time bidirectional communication
//! - **[`coap`]**: Constrained Application Protocol for resource-limited devices
//!
//! ## Design Principles
//!
//! All protocol implementations in this module follow these principles:
//!
//! - **Connection Agnostic**: Work with any type implementing [`Connection`](crate::network::Connection)
//! - **No-std Compatible**: Designed for embedded systems without heap allocation
//! - **Resource Conscious**: Use fixed-size buffers and minimal memory
//! - **Error Handling**: Comprehensive error types for robust applications
//!
//! ## Usage Pattern
//!
//! Most protocol clients follow a similar pattern:
//!
//! 1. Create a connection using your transport layer
//! 2. Wrap it with the protocol client
//! 3. Use protocol-specific methods for communication
//!
//! ```rust,no_run
//! use libiot::network::application::http::{Client, Request, Method};
//! # use libiot::network::Connection;
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
//! // 1. Create connection (implementation-specific)
//! let connection = MockConnection;
//!
//! // 2. Wrap with protocol client
//! let mut client = Client::new(connection);
//!
//! // 3. Use protocol methods
//! let request = Request {
//!     method: Method::Get,
//!     path: "/api/status",
//!     headers: heapless::Vec::new(),
//!     body: None,
//! };
//! // let response = client.request(&request)?;
//! ```

/// CoAP (Constrained Application Protocol) implementation.
///
/// CoAP is a specialized web transfer protocol designed for use with constrained
/// nodes and constrained networks in the Internet of Things.
pub mod coap;

/// HTTP client implementation.
///
/// Provides a simple HTTP/1.1 client suitable for embedded systems,
/// supporting GET and POST methods with custom headers.
pub mod http;

/// MCP (Model Context Protocol) client implementation.
///
/// Enables embedded devices to interact with AI models and language models
/// through a standardized protocol interface.
pub mod mcp;

/// MQTT client implementation.
///
/// Provides an MQTT 3.1.1 client for lightweight publish-subscribe messaging,
/// commonly used in IoT applications.
pub mod mqtt;

/// WebSocket protocol implementation.
///
/// Enables real-time bidirectional communication between embedded devices
/// and web services.
pub mod websocket;
