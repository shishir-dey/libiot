//! MCP handler implementations for common embedded operations

pub mod gpio;
pub mod ping;
pub mod system_info;
pub mod temperature;

pub use gpio::GpioHandler;
pub use ping::PingHandler;
pub use system_info::SystemInfoHandler;
pub use temperature::TemperatureSensorHandler;
