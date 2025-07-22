//! GPIO pin control handler for MCP

use super::super::{HandlerResult, McpError, McpHandler};
use heapless::{FnvIndexMap, String};
use serde::{Deserialize, Serialize};

/// GPIO pin control handler
pub struct GpioHandler {
    // In a real implementation, this would interface with actual GPIO hardware
    pin_states: FnvIndexMap<u8, bool, 16>,
}

#[derive(Deserialize)]
struct GpioArgs {
    pin: u8,
    state: Option<bool>, // None for read, Some(bool) for write
}

#[derive(Serialize)]
struct GpioResult {
    pin: u8,
    state: bool,
}

impl GpioHandler {
    pub fn new() -> Self {
        Self {
            pin_states: FnvIndexMap::new(),
        }
    }
}

impl McpHandler for GpioHandler {
    fn call(&mut self, args: &str) -> HandlerResult {
        // Parse GPIO arguments
        let (gpio_args, _): (GpioArgs, _) =
            serde_json_core::from_str(args).map_err(|_| McpError::InvalidArguments)?;

        match gpio_args.state {
            Some(new_state) => {
                // Set GPIO pin state
                self.pin_states
                    .insert(gpio_args.pin, new_state)
                    .map_err(|_| McpError::ExecutionError)?;

                let result = GpioResult {
                    pin: gpio_args.pin,
                    state: new_state,
                };

                let mut buf = [0u8; 64];
                let serialized_len = serde_json_core::to_slice(&result, &mut buf)
                    .map_err(|_| McpError::ExecutionError)?;

                Ok(Some(
                    String::try_from(
                        core::str::from_utf8(&buf[..serialized_len])
                            .map_err(|_| McpError::ExecutionError)?,
                    )
                    .map_err(|_| McpError::BufferOverflow)?,
                ))
            }
            None => {
                // Read GPIO pin state
                let state = self
                    .pin_states
                    .get(&gpio_args.pin)
                    .copied()
                    .unwrap_or(false);

                let result = GpioResult {
                    pin: gpio_args.pin,
                    state,
                };

                let mut buf = [0u8; 64];
                let serialized_len = serde_json_core::to_slice(&result, &mut buf)
                    .map_err(|_| McpError::ExecutionError)?;

                Ok(Some(
                    String::try_from(
                        core::str::from_utf8(&buf[..serialized_len])
                            .map_err(|_| McpError::ExecutionError)?,
                    )
                    .map_err(|_| McpError::BufferOverflow)?,
                ))
            }
        }
    }
}
