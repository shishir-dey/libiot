//! Model Context Protocol (MCP) Client for Embedded Systems
//!
//! This module provides a lightweight MCP client implementation suitable for
//! microcontrollers and other `no_std` environments. It supports receiving
//! tool/function calls from LLMs and executing them on-device.

#![deny(unsafe_code)]

use heapless::{FnvIndexMap, String};
use serde::{Deserialize, Serialize};

pub mod client;
pub mod handlers;

pub use client::McpClient;

/// Maximum length for function names
pub const MAX_FUNCTION_NAME_LEN: usize = 32;
/// Maximum length for JSON argument strings
pub const MAX_ARGS_LEN: usize = 256;
/// Maximum length for response messages
pub const MAX_RESPONSE_LEN: usize = 128;
/// Maximum number of registered functions
pub const MAX_FUNCTIONS: usize = 16;

/// Core MCP message for function calls
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpMessage<'a> {
    /// The function name to call
    #[serde(borrow)]
    pub function: &'a str,
    /// JSON arguments as a raw string (to be parsed by handlers)
    #[serde(borrow)]
    pub arguments: &'a str,
}

/// Response message for function call results
#[derive(Debug, Clone, Serialize)]
pub struct McpResponse {
    /// Status of the function call
    pub status: ResponseStatus,
    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String<64>>,
    /// Optional result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String<MAX_RESPONSE_LEN>>,
}

/// Status codes for MCP responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    /// Function executed successfully
    Ok,
    /// Function execution failed
    Error,
    /// Function not found
    NotFound,
    /// Invalid arguments provided
    InvalidArgs,
}

/// Result type for function handlers
pub type HandlerResult = Result<Option<String<MAX_RESPONSE_LEN>>, McpError>;

/// Error types for MCP operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpError {
    /// JSON parsing failed
    ParseError,
    /// Function not found in registry
    FunctionNotFound,
    /// Invalid arguments provided to function
    InvalidArguments,
    /// Function execution failed
    ExecutionError,
    /// Buffer overflow (message too large)
    BufferOverflow,
}

/// Function handler trait for MCP functions
pub trait McpHandler {
    /// Execute the function with given JSON arguments
    fn call(&mut self, args: &str) -> HandlerResult;
}

/// Function registry for compile-time function registration
pub struct FunctionRegistry<H> {
    handlers: FnvIndexMap<String<MAX_FUNCTION_NAME_LEN>, H, MAX_FUNCTIONS>,
}

impl<H: McpHandler> FunctionRegistry<H> {
    /// Create a new function registry
    pub fn new() -> Self {
        Self {
            handlers: FnvIndexMap::new(),
        }
    }

    /// Register a function handler
    pub fn register(&mut self, name: &str, handler: H) -> Result<(), McpError> {
        let key = String::try_from(name).map_err(|_| McpError::BufferOverflow)?;
        self.handlers
            .insert(key, handler)
            .map_err(|_| McpError::BufferOverflow)?;
        Ok(())
    }

    /// Execute a function by name
    pub fn execute(&mut self, function: &str, args: &str) -> McpResponse {
        // Find the handler by comparing string contents
        let mut found_handler = None;
        for (key, _) in &self.handlers {
            if key.as_str() == function {
                found_handler = Some(key.clone());
                break;
            }
        }

        match found_handler.and_then(|key| self.handlers.get_mut(&key)) {
            Some(handler) => match handler.call(args) {
                Ok(result) => McpResponse {
                    status: ResponseStatus::Ok,
                    error: None,
                    result,
                },
                Err(McpError::InvalidArguments) => McpResponse {
                    status: ResponseStatus::InvalidArgs,
                    error: Some(String::try_from("Invalid arguments").unwrap_or_default()),
                    result: None,
                },
                Err(_) => McpResponse {
                    status: ResponseStatus::Error,
                    error: Some(String::try_from("Execution failed").unwrap_or_default()),
                    result: None,
                },
            },
            None => McpResponse {
                status: ResponseStatus::NotFound,
                error: Some(String::try_from("Function not found").unwrap_or_default()),
                result: None,
            },
        }
    }
}

impl<H: McpHandler> Default for FunctionRegistry<H> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for McpError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            McpError::ParseError => defmt::write!(f, "ParseError"),
            McpError::FunctionNotFound => defmt::write!(f, "FunctionNotFound"),
            McpError::InvalidArguments => defmt::write!(f, "InvalidArguments"),
            McpError::ExecutionError => defmt::write!(f, "ExecutionError"),
            McpError::BufferOverflow => defmt::write!(f, "BufferOverflow"),
        }
    }
}

/// Macro to help register multiple functions at once
#[macro_export]
macro_rules! register_mcp_functions {
    ($registry:expr, $(($name:expr, $handler:expr)),+ $(,)?) => {
        $(
            $registry.register($name, $handler).expect("Failed to register MCP function");
        )+
    };
}
