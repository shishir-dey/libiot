//! MCP Client implementation for embedded systems

use super::*;
use crate::network::{Connection, error::Error as NetworkError};
use heapless::Vec;

/// MCP Client that works over any connection type
pub struct McpClient<C, H>
where
    C: Connection,
    H: McpHandler,
{
    connection: C,
    registry: FunctionRegistry<H>,
    buffer: Vec<u8, 1024>,
}

impl<C, H> McpClient<C, H>
where
    C: Connection,
    H: McpHandler,
{
    /// Create a new MCP client with a connection and function registry
    pub fn new(connection: C, registry: FunctionRegistry<H>) -> Self {
        Self {
            connection,
            registry,
            buffer: Vec::new(),
        }
    }

    /// Process incoming MCP messages and return responses
    pub fn process_message(&mut self) -> Result<(), NetworkError> {
        // Clear buffer for new message
        self.buffer.clear();

        // Read incoming data
        let mut temp_buf = [0u8; 256];
        loop {
            match self.connection.read(&mut temp_buf) {
                Ok(0) => break, // No more data
                Ok(n) => {
                    if self.buffer.extend_from_slice(&temp_buf[..n]).is_err() {
                        return Err(NetworkError::ReadError);
                    }

                    // Check if we have a complete JSON message
                    if self.has_complete_message() {
                        break;
                    }
                }
                Err(_) => return Err(NetworkError::ReadError),
            }
        }

        if self.buffer.is_empty() {
            return Ok(());
        }

        // Parse and handle the message
        let response = self.handle_message();

        // Send response back
        self.send_response(&response)
    }

    /// Check if buffer contains a complete JSON message
    fn has_complete_message(&self) -> bool {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for &byte in &self.buffer {
            if escape_next {
                escape_next = false;
                continue;
            }

            match byte {
                b'\\' if in_string => escape_next = true,
                b'"' => in_string = !in_string,
                b'{' if !in_string => brace_count += 1,
                b'}' if !in_string => {
                    if brace_count > 0 {
                        brace_count -= 1;
                        if brace_count == 0 {
                            return true;
                        }
                    }
                    // If brace_count is 0 and we encounter '}', ignore it
                    // as it indicates malformed JSON (extra closing brace)
                }
                _ => {}
            }
        }

        false
    }

    /// Parse and handle an MCP message
    fn handle_message(&mut self) -> McpResponse {
        // Try to parse the JSON message
        let message_str = match core::str::from_utf8(&self.buffer) {
            Ok(s) => s,
            Err(_) => {
                return McpResponse {
                    status: ResponseStatus::Error,
                    error: Some(heapless::String::try_from("Invalid UTF-8").unwrap_or_default()),
                    result: None,
                };
            }
        };

        // Parse the MCP message
        match serde_json_core::from_str::<McpMessage>(message_str) {
            Ok((message, _)) => {
                // Execute the function
                self.registry.execute(message.function, message.arguments)
            }
            Err(_) => McpResponse {
                status: ResponseStatus::Error,
                error: Some(heapless::String::try_from("JSON parse error").unwrap_or_default()),
                result: None,
            },
        }
    }

    /// Send response back over the connection
    fn send_response(&mut self, response: &McpResponse) -> Result<(), NetworkError> {
        // Serialize response to JSON
        let mut response_buf = [0u8; 512];
        match serde_json_core::to_slice(response, &mut response_buf) {
            Ok(len) => {
                // Send the response
                self.connection
                    .write(&response_buf[..len])
                    .map_err(|_| NetworkError::WriteError)?;
                self.connection
                    .flush()
                    .map_err(|_| NetworkError::WriteError)?;
                Ok(())
            }
            Err(_) => Err(NetworkError::WriteError),
        }
    }

    /// Get a mutable reference to the function registry
    pub fn registry_mut(&mut self) -> &mut FunctionRegistry<H> {
        &mut self.registry
    }

    /// Get the underlying connection
    pub fn connection(&self) -> &C {
        &self.connection
    }

    /// Get a mutable reference to the underlying connection
    pub fn connection_mut(&mut self) -> &mut C {
        &mut self.connection
    }
}
