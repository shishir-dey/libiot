//! Temperature sensor reading handler for MCP

use super::super::{HandlerResult, McpError, McpHandler};
use heapless::String;
use serde::{Deserialize, Serialize};

/// Temperature sensor reading handler
pub struct TemperatureSensorHandler {
    // In real implementation, this would interface with actual sensor hardware
    last_reading: f32,
}

#[derive(Deserialize)]
struct TempArgs {
    unit: Option<String<16>>, // "celsius" or "fahrenheit"
}

#[derive(Serialize)]
struct TempResult {
    temperature: f32,
    unit: String<10>,
}

impl TemperatureSensorHandler {
    pub fn new() -> Self {
        Self {
            last_reading: 25.0, // Default room temperature
        }
    }

    /// Simulate reading temperature (in real implementation, read from actual sensor)
    fn read_temperature(&mut self) -> f32 {
        // Simulate some variation
        self.last_reading += 0.1;
        if self.last_reading > 30.0 {
            self.last_reading = 20.0;
        }
        self.last_reading
    }
}

impl McpHandler for TemperatureSensorHandler {
    fn call(&mut self, args: &str) -> HandlerResult {
        let temp_args: TempArgs = if args.trim().is_empty() {
            TempArgs { unit: None }
        } else {
            serde_json_core::from_str(args)
                .map_err(|_| McpError::InvalidArguments)?
                .0
        };

        let temp_celsius = self.read_temperature();
        let (temperature, unit) = match temp_args.unit.as_deref() {
            Some("fahrenheit") | Some("f") => (temp_celsius * 9.0 / 5.0 + 32.0, "fahrenheit"),
            _ => (temp_celsius, "celsius"),
        };

        let result = TempResult {
            temperature,
            unit: String::try_from(unit).map_err(|_| McpError::BufferOverflow)?,
        };

        let mut buf = [0u8; 128];
        let serialized_len =
            serde_json_core::to_slice(&result, &mut buf).map_err(|_| McpError::ExecutionError)?;

        Ok(Some(
            String::try_from(
                core::str::from_utf8(&buf[..serialized_len])
                    .map_err(|_| McpError::ExecutionError)?,
            )
            .map_err(|_| McpError::BufferOverflow)?,
        ))
    }
}
