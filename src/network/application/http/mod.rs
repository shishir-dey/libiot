//! HTTP/1.1 protocol implementation for embedded systems.
//!
//! This module provides a lightweight HTTP client implementation designed specifically
//! for embedded systems and `no_std` environments. It focuses on simplicity,
//! predictable memory usage, and compatibility with resource-constrained devices.
//!
//! # Features
//!
//! - HTTP/1.1 protocol compliance
//! - Synchronous request/response model
//! - Fixed-size buffers for predictable memory usage
//! - Support for custom headers
//! - GET and POST method support
//! - Connection reuse capability
//!
//! # Usage
//!
//! The main entry point is the [`client::Client`] which works with any connection
//! type implementing the [`crate::network::Connection`] trait.
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
//! let connection = MockConnection;
//! let mut client = Client::new(connection);
//!
//! let request = Request {
//!     method: Method::Get,
//!     path: "/api/status",
//!     headers: heapless::Vec::new(),
//!     body: None,
//! };
//!
//! // let response = client.request(&request)?;
//! ```

/// HTTP client implementation and supporting types.
///
/// Contains the main [`Client`](client::Client) struct and all related types
/// for making HTTP requests and handling responses.
pub mod client;
