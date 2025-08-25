//! # libiot - Rust IoT SDK
//!
//! A comprehensive Rust SDK that enables any IoT device to securely connect to the cloud,
//! manage data, and interact with cloud services. This library is designed for embedded
//! systems and supports `no_std` environments.
//!
//! ## Features
//!
//! ### Network Protocols
//! - **HTTP Client**: RESTful API communication
//! - **MQTT Client**: Lightweight publish-subscribe messaging
//! - **MCP (Model Context Protocol)**: AI/LLM integration for embedded systems
//! - **WebSocket**: Real-time bidirectional communication
//! - **CoAP**: Constrained Application Protocol for IoT
//!
//! ### Storage Abstraction
//! - Unified storage interface for various memory types
//! - Support for EEPROM, Flash, SD cards, and RAM
//! - Async and sync operation modes
//!
//! ### System Utilities
//! - Command shell interface for embedded devices
//! - Extensible command system with built-in help
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! libiot = "0.1.0"
//! ```
//!
//! ### Basic HTTP Client Example
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
//!     path: "/api/data",
//!     headers: heapless::Vec::new(),
//!     body: None,
//! };
//!
//! // let response = client.request(&request)?;
//! ```
//!
//! ### MQTT Client Example
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
//!     client_id: "my_device",
//!     keep_alive_seconds: 60,
//!     clean_session: true,
//! };
//!
//! // let mut client = Client::connect(connection, options)?;
//! // client.publish("sensors/temperature", b"23.5", QoS::AtMostOnce)?;
//! ```
//!
//! ## Platform Support
//!
//! This library is designed to work on:
//! - Embedded microcontrollers (ARM Cortex-M, RISC-V, etc.)
//! - Linux-based IoT devices (Raspberry Pi, etc.)
//! - Any platform supporting Rust's `core` library
//!
//! ## Optional Features
//!
//! - `std`: Enable standard library support (default: disabled)
//! - `async`: Enable async/await support for non-blocking operations
//! - `defmt`: Enable defmt logging support for embedded debugging

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![warn(missing_debug_implementations)]
#![doc(html_root_url = "https://shishir-dey.github.io/libiot/")]

/// Network abstraction layer providing protocol implementations and connection management.
///
/// This module contains implementations for various network protocols commonly used
/// in IoT applications, including HTTP, MQTT, WebSocket, CoAP, and MCP.
pub mod network;

/// Storage abstraction layer for various memory and storage devices.
///
/// Provides unified interfaces for different storage technologies including
/// Flash memory, EEPROM, SD cards, and RAM-based storage.
pub mod storage;

/// System utilities for embedded devices.
///
/// Contains tools like command shell interfaces and system management utilities
/// that are commonly needed in IoT device firmware.
pub mod system;

/// Over-the-air (OTA) update logic combining network and storage layers.
pub mod ota;

pub mod gps;
