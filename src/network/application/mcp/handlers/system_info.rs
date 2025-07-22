//! System information handler for MCP

use super::super::{HandlerResult, McpError, McpHandler};
use heapless::String;
use serde::Serialize;

/// System information handler
pub struct SystemInfoHandler {
    device_id: String<32>,
}

#[derive(Serialize)]
struct SystemInfo<'a> {
    device_id: &'a str,
    uptime_ms: u32,
    free_memory: u32,
}

impl SystemInfoHandler {
    pub fn new(device_id: &str) -> Result<Self, McpError> {
        Ok(Self {
            device_id: String::try_from(device_id).map_err(|_| McpError::BufferOverflow)?,
        })
    }
}

impl McpHandler for SystemInfoHandler {
    fn call(&mut self, _args: &str) -> HandlerResult {
        // In real implementation, these would be actual system readings
        let info = SystemInfo {
            device_id: &self.device_id,
            uptime_ms: 1000000, // Placeholder
            free_memory: 32768, // Placeholder
        };

        let mut buf = [0u8; 128];
        let serialized_len =
            serde_json_core::to_slice(&info, &mut buf).map_err(|_| McpError::ExecutionError)?;

        Ok(Some(
            String::try_from(
                core::str::from_utf8(&buf[..serialized_len])
                    .map_err(|_| McpError::ExecutionError)?,
            )
            .map_err(|_| McpError::BufferOverflow)?,
        ))
    }
}
