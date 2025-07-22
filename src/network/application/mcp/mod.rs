//! Model Context Protocol (MCP) Client for Embedded Systems
//!
//! This module provides a lightweight MCP client implementation suitable for
//! microcontrollers and other `no_std` environments. It supports receiving
//! tool/function calls from LLMs and executing them on-device.
//!
//! # Overview
//!
//! Model Context Protocol (MCP) is a communication protocol designed to enable
//! AI models and language models to interact with external tools and services.
//! This implementation allows embedded devices to act as MCP servers, exposing
//! device capabilities to AI systems.
//!
//! ## Key Concepts
//!
//! - **Functions**: Callable operations exposed by the device
//! - **Handlers**: Rust code that implements function logic
//! - **Registry**: A collection of registered functions
//! - **Messages**: JSON-based communication between client and AI model
//!
//! # Architecture
//!
//! The MCP implementation consists of several key components:
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │   AI Model  │───▶│ MCP Client  │───▶│  Function   │
//! │             │    │             │    │  Registry   │
//! └─────────────┘    └─────────────┘    └─────────────┘
//!                            │                   │
//!                            ▼                   ▼
//!                    ┌─────────────┐    ┌─────────────┐
//!                    │ Connection  │    │  Handlers   │
//!                    │   Layer     │    │             │
//!                    └─────────────┘    └─────────────┘
//! ```
//!
//! # Features
//!
//! - **Zero-allocation**: Uses fixed-size buffers for predictable memory usage
//! - **Type Safety**: Strongly typed function signatures and error handling
//! - **Extensible**: Easy to add custom functions and handlers
//! - **Connection Agnostic**: Works with any transport implementing [`Connection`](crate::network::Connection)
//! - **JSON Communication**: Standard JSON message format for compatibility
//!
//! # Usage Examples
//!
//! ## Basic Function Registration and Execution
//!
//! ```rust,no_run
//! use libiot::network::application::mcp::{
//!     FunctionRegistry, McpHandler, HandlerResult, register_mcp_functions
//! };
//! use libiot::network::application::mcp::handlers::PingHandler;
//!
//! // Create a function registry
//! let mut registry = FunctionRegistry::new();
//!
//! // Register built-in handlers
//! register_mcp_functions!(registry,
//!     ("ping", PingHandler),
//!     ("system_info", libiot::network::application::mcp::handlers::SystemInfoHandler),
//! );
//!
//! // Execute a function
//! let response = registry.execute("ping", r#"{"message": "Hello"}"#);
//! ```
//!
//! ## Custom Function Handler
//!
//! ```rust
//! use libiot::network::application::mcp::{McpHandler, HandlerResult, McpError};
//! use heapless::String;
//!
//! struct TemperatureHandler {
//!     current_temp: f32,
//! }
//!
//! impl McpHandler for TemperatureHandler {
//!     fn call(&mut self, args: &str) -> HandlerResult {
//!         // Parse arguments and return current temperature
//!         let result = format!("Temperature: {:.1}°C", self.current_temp);
//!         Ok(Some(String::try_from(result.as_str()).map_err(|_| McpError::BufferOverflow)?))
//!     }
//! }
//! ```
//!
//! ## MCP Client Usage
//!
//! ```rust,no_run
//! use libiot::network::application::mcp::{McpClient, FunctionRegistry};
//! use libiot::network::application::mcp::handlers::PingHandler;
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
//! let mut registry = FunctionRegistry::new();
//! registry.register("ping", PingHandler).unwrap();
//!
//! let mut client = McpClient::new(connection, registry);
//!
//! // Process incoming MCP messages
//! // loop {
//! //     if let Err(e) = client.process_message() {
//! //         println!("Error processing message: {:?}", e);
//! //     }
//! // }
//! ```

#![deny(unsafe_code)]

use heapless::{FnvIndexMap, String};
use serde::{Deserialize, Serialize};

pub mod client;
pub mod handlers;

pub use client::McpClient;

/// Maximum length for function names in characters.
///
/// This limits the length of function identifiers to keep memory usage
/// predictable in embedded environments.
pub const MAX_FUNCTION_NAME_LEN: usize = 32;

/// Maximum length for JSON argument strings in bytes.
///
/// This constrains the size of function arguments to prevent memory
/// overflow while still allowing reasonably complex parameter structures.
pub const MAX_ARGS_LEN: usize = 256;

/// Maximum length for response messages in characters.
///
/// Function handlers can return responses up to this length. Longer
/// responses should be chunked or simplified.
pub const MAX_RESPONSE_LEN: usize = 128;

/// Maximum number of registered functions in the registry.
///
/// This defines how many different functions can be registered with
/// a single MCP client. Increase if more functions are needed.
pub const MAX_FUNCTIONS: usize = 16;

/// Core MCP message structure for function calls.
///
/// This represents an incoming request from an AI model to execute a specific
/// function with provided arguments. The message follows the MCP specification
/// for function invocation.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mcp::McpMessage;
///
/// // Parsing would typically be done automatically by the client
/// let json = r#"{"function": "get_temperature", "arguments": "{\"unit\": \"celsius\"}"}"#;
/// // let message: McpMessage = serde_json::from_str(json)?;
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpMessage<'a> {
    /// The function name to call.
    ///
    /// This should match a function registered in the [`FunctionRegistry`].
    /// Function names are case-sensitive and must be valid UTF-8 strings.
    #[serde(borrow)]
    pub function: &'a str,

    /// JSON arguments as a raw string to be parsed by handlers.
    ///
    /// Arguments are provided as a JSON string that individual handlers
    /// will parse according to their specific requirements. This approach
    /// allows for flexible argument handling without requiring complex
    /// generic type parameters.
    #[serde(borrow)]
    pub arguments: &'a str,
}

/// Response message structure for function call results.
///
/// This structure is serialized to JSON and sent back to the AI model
/// after function execution. It includes the execution status, any error
/// information, and optional result data.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mcp::{McpResponse, ResponseStatus};
/// use heapless::String;
///
/// let success_response = McpResponse {
///     status: ResponseStatus::Ok,
///     error: None,
///     result: Some(String::try_from("Operation completed").unwrap()),
/// };
///
/// let error_response = McpResponse {
///     status: ResponseStatus::Error,
///     error: Some(String::try_from("Invalid parameters").unwrap()),
///     result: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct McpResponse {
    /// Status of the function call execution.
    pub status: ResponseStatus,

    /// Optional error message when status indicates failure.
    ///
    /// This field is only included in the JSON output when an error occurs,
    /// keeping successful responses concise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String<64>>,

    /// Optional result data from successful function execution.
    ///
    /// This field contains the actual return value from the function handler.
    /// It's omitted from JSON when the function doesn't return data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String<MAX_RESPONSE_LEN>>,
}

/// Status codes for MCP function execution responses.
///
/// These status codes indicate the outcome of function execution and help
/// the AI model understand whether the operation succeeded and how to
/// handle any failures.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mcp::ResponseStatus;
///
/// // Match on response status
/// let status = ResponseStatus::Ok;
/// match status {
///     ResponseStatus::Ok => println!("Function executed successfully"),
///     ResponseStatus::Error => println!("Function execution failed"),
///     ResponseStatus::NotFound => println!("Function not found"),
///     ResponseStatus::InvalidArgs => println!("Invalid arguments provided"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    /// Function executed successfully.
    ///
    /// The function completed without errors and may have returned result data.
    Ok,

    /// Function execution failed due to an internal error.
    ///
    /// This indicates an error occurred during function execution, such as
    /// a runtime error, resource unavailable, or other execution failure.
    Error,

    /// Requested function was not found in the registry.
    ///
    /// The function name provided in the request doesn't match any registered
    /// function. This usually indicates a typo or outdated function name.
    NotFound,

    /// Invalid arguments were provided to the function.
    ///
    /// The arguments couldn't be parsed or don't match the function's
    /// expected parameter format. This helps distinguish between execution
    /// errors and input validation errors.
    InvalidArgs,
}

/// Result type for MCP function handlers.
///
/// This type alias simplifies the return type for function implementations.
/// Handlers return either a successful result with optional data, or an
/// error indicating what went wrong.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mcp::{HandlerResult, McpError};
/// use heapless::String;
///
/// fn example_handler() -> HandlerResult {
///     // Success with result data
///     Ok(Some(String::try_from("Success!").unwrap()))
/// }
///
/// fn another_handler() -> HandlerResult {
///     // Success without result data
///     Ok(None)
/// }
///
/// fn failing_handler() -> HandlerResult {
///     // Error case
///     Err(McpError::InvalidArguments)
/// }
/// ```
pub type HandlerResult = Result<Option<String<MAX_RESPONSE_LEN>>, McpError>;

/// Error types for MCP operations.
///
/// These errors cover the various failure modes that can occur during
/// MCP message processing, function execution, and system operations.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mcp::McpError;
///
/// // Handle different error types
/// let error = McpError::ParseError;
/// match error {
///     McpError::ParseError => println!("Failed to parse JSON message"),
///     McpError::FunctionNotFound => println!("Unknown function requested"),
///     McpError::InvalidArguments => println!("Bad function arguments"),
///     McpError::ExecutionError => println!("Function execution failed"),
///     McpError::BufferOverflow => println!("Response too large for buffer"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpError {
    /// JSON parsing failed.
    ///
    /// The incoming message couldn't be parsed as valid JSON or doesn't
    /// match the expected MCP message structure.
    ParseError,

    /// Function not found in registry.
    ///
    /// The requested function name doesn't exist in the function registry.
    /// This is converted to a `NotFound` response status.
    FunctionNotFound,

    /// Invalid arguments provided to function.
    ///
    /// The function arguments couldn't be parsed or validated by the handler.
    /// This is converted to an `InvalidArgs` response status.
    InvalidArguments,

    /// Function execution failed.
    ///
    /// An error occurred during function execution, such as hardware failure,
    /// resource unavailability, or other runtime errors.
    ExecutionError,

    /// Buffer overflow (message too large).
    ///
    /// The response message, function name, or arguments exceed the maximum
    /// allowed size for the embedded buffers.
    BufferOverflow,
}

/// Function handler trait for MCP functions.
///
/// This trait must be implemented by all MCP function handlers. It provides
/// a standardized interface for executing functions with JSON arguments
/// and returning structured results.
///
/// # Implementation Guidelines
///
/// - Parse arguments carefully and return `InvalidArguments` for malformed input
/// - Keep responses concise to fit within buffer limits
/// - Use appropriate error types to help with debugging
/// - Ensure thread safety if the handler maintains state
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mcp::{McpHandler, HandlerResult, McpError};
/// use heapless::String;
///
/// struct CounterHandler {
///     count: u32,
/// }
///
/// impl McpHandler for CounterHandler {
///     fn call(&mut self, args: &str) -> HandlerResult {
///         // Simple increment function
///         self.count += 1;
///         let result = format!("Count: {}", self.count);
///         Ok(Some(String::try_from(result.as_str()).map_err(|_| McpError::BufferOverflow)?))
///     }
/// }
/// ```
pub trait McpHandler {
    /// Execute the function with given JSON arguments.
    ///
    /// This method is called when the AI model requests execution of this
    /// function. The implementation should parse the arguments, execute the
    /// required logic, and return either a success result or an appropriate error.
    ///
    /// # Arguments
    ///
    /// * `args` - JSON string containing function arguments
    ///
    /// # Returns
    ///
    /// * `Ok(Some(result))` - Function succeeded with result data
    /// * `Ok(None)` - Function succeeded without result data
    /// * `Err(error)` - Function failed with specific error type
    ///
    /// # Error Handling
    ///
    /// Return `McpError::InvalidArguments` if the arguments can't be parsed
    /// or are invalid. Return `McpError::ExecutionError` for runtime failures.
    /// Return `McpError::BufferOverflow` if the response is too large.
    fn call(&mut self, args: &str) -> HandlerResult;
}

/// Function registry for compile-time function registration.
///
/// The registry manages a collection of MCP functions that can be called
/// by AI models. It provides type-safe function registration and execution
/// with efficient lookup using a hash map.
///
/// # Type Parameters
///
/// * `H` - The handler type implementing [`McpHandler`]
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mcp::{FunctionRegistry, McpHandler, HandlerResult};
///
/// struct MyHandler;
/// impl McpHandler for MyHandler {
///     fn call(&mut self, _args: &str) -> HandlerResult { Ok(None) }
/// }
///
/// let mut registry = FunctionRegistry::new();
/// registry.register("my_function", MyHandler).unwrap();
///
/// // Execute function
/// let response = registry.execute("my_function", "{}");
/// ```
pub struct FunctionRegistry<H> {
    handlers: FnvIndexMap<String<MAX_FUNCTION_NAME_LEN>, H, MAX_FUNCTIONS>,
}

impl<H: McpHandler> FunctionRegistry<H> {
    /// Create a new empty function registry.
    ///
    /// The registry starts with no registered functions. Use [`register`](Self::register)
    /// to add functions before attempting to execute them.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::network::application::mcp::{FunctionRegistry, McpHandler, HandlerResult};
    ///
    /// struct DummyHandler;
    /// impl McpHandler for DummyHandler {
    ///     fn call(&mut self, _args: &str) -> HandlerResult { Ok(None) }
    /// }
    ///
    /// let registry: FunctionRegistry<DummyHandler> = FunctionRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            handlers: FnvIndexMap::new(),
        }
    }

    /// Register a function handler with a given name.
    ///
    /// Associates a function name with a handler implementation. The name
    /// will be used to route incoming function calls to the correct handler.
    /// Function names must be unique within a registry.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique function name (max 32 characters)
    /// * `handler` - Handler implementation for this function
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Function registered successfully
    /// * `Err(McpError::BufferOverflow)` - Name too long or registry full
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::network::application::mcp::{FunctionRegistry, McpHandler, HandlerResult};
    ///
    /// struct EchoHandler;
    /// impl McpHandler for EchoHandler {
    ///     fn call(&mut self, args: &str) -> HandlerResult {
    ///         // Echo back the arguments
    ///         Ok(Some(heapless::String::try_from(args).map_err(|_|
    ///             libiot::network::application::mcp::McpError::BufferOverflow)?))
    ///     }
    /// }
    ///
    /// let mut registry = FunctionRegistry::new();
    /// registry.register("echo", EchoHandler).unwrap();
    /// ```
    pub fn register(&mut self, name: &str, handler: H) -> Result<(), McpError> {
        let key = String::try_from(name).map_err(|_| McpError::BufferOverflow)?;
        self.handlers
            .insert(key, handler)
            .map_err(|_| McpError::BufferOverflow)?;
        Ok(())
    }

    /// Execute a function by name with provided arguments.
    ///
    /// Looks up the function handler by name and executes it with the given
    /// arguments. Returns a structured response that can be serialized and
    /// sent back to the AI model.
    ///
    /// # Arguments
    ///
    /// * `function` - Name of the function to execute
    /// * `args` - JSON string containing function arguments
    ///
    /// # Returns
    ///
    /// An [`McpResponse`] containing the execution result, status, and any
    /// error information. The response is always returned, even for errors,
    /// to provide structured feedback to the AI model.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::network::application::mcp::{FunctionRegistry, ResponseStatus};
    /// use libiot::network::application::mcp::handlers::PingHandler;
    ///
    /// let mut registry = FunctionRegistry::new();
    /// registry.register("ping", PingHandler).unwrap();
    ///
    /// let response = registry.execute("ping", r#"{"message": "test"}"#);
    /// assert_eq!(response.status, ResponseStatus::Ok);
    ///
    /// let not_found = registry.execute("unknown", "{}");
    /// assert_eq!(not_found.status, ResponseStatus::NotFound);
    /// ```
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

/// Convenience macro to register multiple MCP functions at once.
///
/// This macro simplifies the process of registering multiple functions
/// with a registry by handling the repetitive registration calls and
/// error checking.
///
/// # Syntax
///
/// ```rust,ignore
/// register_mcp_functions!(registry,
///     ("function_name", HandlerType),
///     ("another_function", AnotherHandlerType),
///     // ... more functions
/// );
/// ```
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mcp::{FunctionRegistry, register_mcp_functions};
/// use libiot::network::application::mcp::handlers::{PingHandler, SystemInfoHandler};
///
/// let mut registry = FunctionRegistry::new();
///
/// register_mcp_functions!(registry,
///     ("ping", PingHandler),
///     ("system_info", SystemInfoHandler),
/// );
/// ```
///
/// # Panics
///
/// The macro will panic if any function registration fails, which typically
/// happens when the function name is too long or the registry is full.
/// This is designed for setup-time registration where failures indicate
/// configuration errors that should be caught during development.
#[macro_export]
macro_rules! register_mcp_functions {
    ($registry:expr, $(($name:expr, $handler:expr)),+ $(,)?) => {
        $(
            $registry.register($name, $handler).expect("Failed to register MCP function");
        )+
    };
}
