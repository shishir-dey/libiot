//! MQTT 3.1.1 protocol implementation for embedded systems.
//!
//! This module provides a complete MQTT 3.1.1 client implementation designed for
//! embedded systems and `no_std` environments. MQTT (Message Queuing Telemetry Transport)
//! is a lightweight publish-subscribe messaging protocol ideal for IoT applications.
//!
//! # Protocol Overview
//!
//! MQTT uses a publish-subscribe pattern where:
//! - **Publishers** send messages to topics
//! - **Subscribers** receive messages from topics they're interested in
//! - **Brokers** route messages between publishers and subscribers
//!
//! # Key Features
//!
//! - MQTT 3.1.1 specification compliance
//! - Quality of Service (QoS) levels 0, 1, and 2
//! - Clean session and persistent session support
//! - Topic filtering with wildcards
//! - Keep-alive mechanism for connection monitoring
//! - Minimal memory footprint for embedded systems
//!
//! # Usage
//!
//! The main entry point is the [`client::Client`] which provides methods for
//! connecting, publishing, subscribing, and receiving messages.
//!
//! ```rust,no_run
//! use libiot::network::application::mqtt::{Client, Options, QoS};
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
//! let options = Options {
//!     client_id: "iot_device_123",
//!     keep_alive_seconds: 60,
//!     clean_session: true,
//! };
//!
//! // let mut client = Client::connect(connection, options)?;
//! // client.subscribe("sensors/+", QoS::AtLeastOnce)?;
//! // client.publish("status", b"online", QoS::AtMostOnce)?;
//! ```

/// MQTT client implementation and supporting types.
///
/// Contains the main [`Client`](client::Client) struct and all related types
/// for MQTT communication, including message structures, configuration options,
/// and Quality of Service definitions.
pub mod client;
