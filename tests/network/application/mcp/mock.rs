//! Mock connection implementation for MCP testing

use heapless::Vec;
use libiot::network::{Close, Connection, Read, Write};

/// Mock connection for testing MCP client functionality
pub struct MockConnection {
    data: &'static [u8],
    read_pos: usize,
    pub writes: Vec<u8, 1024>,
}

impl MockConnection {
    /// Create a new mock connection with predefined data to read
    pub fn new(data: &'static [u8]) -> Self {
        Self {
            data,
            read_pos: 0,
            writes: Vec::new(),
        }
    }

    /// Get the data that was written to this connection
    pub fn written_data(&self) -> &[u8] {
        &self.writes
    }
}

impl Read for MockConnection {
    type Error = libiot::network::error::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.read_pos >= self.data.len() {
            return Ok(0);
        }

        let remaining = self.data.len() - self.read_pos;
        let to_read = core::cmp::min(buf.len(), remaining);

        buf[..to_read].copy_from_slice(&self.data[self.read_pos..self.read_pos + to_read]);
        self.read_pos += to_read;

        Ok(to_read)
    }
}

impl Write for MockConnection {
    type Error = libiot::network::error::Error;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.writes
            .extend_from_slice(buf)
            .map_err(|_| libiot::network::error::Error::WriteError)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Close for MockConnection {
    type Error = libiot::network::error::Error;

    fn close(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Connection for MockConnection {}
