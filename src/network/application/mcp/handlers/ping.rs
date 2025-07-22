//! Simple ping handler for MCP connectivity testing

use super::super::{HandlerResult, McpError, McpHandler};
use heapless::String;

/// Simple ping handler for connectivity testing
pub struct PingHandler;

impl McpHandler for PingHandler {
    fn call(&mut self, _args: &str) -> HandlerResult {
        Ok(Some(
            String::try_from(r#"{"message":"pong"}"#).map_err(|_| McpError::BufferOverflow)?,
        ))
    }
}
