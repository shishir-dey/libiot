//! System utilities for embedded devices.
//!
//! This module provides essential system-level utilities and interfaces commonly
//! needed in embedded IoT applications. It focuses on providing lightweight,
//! `no_std` compatible tools for device management and user interaction.
//!
//! # Available Utilities
//!
//! - **[`shell`]**: Command-line interface implementation for embedded systems
//!
//! # Design Principles
//!
//! - **Embedded-First**: All utilities are designed for resource-constrained environments
//! - **Zero-Allocation**: Fixed-size buffers and stack-based operations
//! - **Configurable**: Features can be enabled/disabled as needed
//! - **Portable**: Works across different embedded platforms and architectures
//!
//! # Usage
//!
//! The system utilities are designed to be used independently or together
//! to build complete embedded applications:
//!
//! ```rust,no_run
//! use libiot::system::shell::{Shell, ShellResult};
//!
//! // Set up a command shell for device interaction
//! let mut shell = Shell::new();
//! shell.set_output_function(|text| {
//!     // Send to UART or other output
//!     print!("{}", text);
//! });
//!
//! // Register device-specific commands
//! shell.register_command("status", "Show device status", |_, _| {
//!     println!("Device: Online");
//!     ShellResult::Ok
//! }).unwrap();
//! ```

/// Command shell interface for embedded systems.
///
/// Provides a complete command-line interface implementation with support for
/// command registration, argument parsing, help system, and interactive input processing.
pub mod shell;
