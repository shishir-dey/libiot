use libiot::network::error::Error;
use libiot::network::*;

pub mod client;

const MOCK_BUFFER_SIZE: usize = 256;

#[derive(Debug)]
struct MockConnection {
    read_buffer: [u8; MOCK_BUFFER_SIZE],
    write_buffer: [u8; MOCK_BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
    is_open: bool,
}

impl MockConnection {
    fn new() -> Self {
        Self {
            read_buffer: [0; MOCK_BUFFER_SIZE],
            write_buffer: [0; MOCK_BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            is_open: true,
        }
    }

    /// Helper for tests to inject data into the connection's read buffer
    fn set_read_data(&mut self, data: &[u8]) {
        let len = data.len().min(MOCK_BUFFER_SIZE);
        self.read_buffer[..len].copy_from_slice(&data[..len]);
        self.read_pos = len;
    }
}

impl Read for MockConnection {
    type Error = Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if !self.is_open {
            return Err(Error::NotOpen);
        }
        let readable = self.read_pos;
        let len = buf.len().min(readable);
        buf[..len].copy_from_slice(&self.read_buffer[..len]);

        // Shift remaining data
        self.read_buffer.copy_within(len..readable, 0);
        self.read_pos -= len;

        Ok(len)
    }
}

impl Write for MockConnection {
    type Error = Error;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if !self.is_open {
            return Err(Error::NotOpen);
        }
        let writeable = MOCK_BUFFER_SIZE - self.write_pos;
        let len = buf.len().min(writeable);
        self.write_buffer[self.write_pos..self.write_pos + len].copy_from_slice(&buf[..len]);
        self.write_pos += len;
        Ok(len)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        if !self.is_open {
            return Err(Error::NotOpen);
        }
        // In this mock, flush does nothing.
        Ok(())
    }
}

impl Close for MockConnection {
    type Error = Error;

    fn close(mut self) -> Result<(), Self::Error> {
        if !self.is_open {
            // This case might not be reachable if close consumes self,
            // but included for robustness.
            return Err(Error::NotOpen);
        }
        self.is_open = false;
        Ok(())
    }
}

// This is needed to satisfy the trait bound on Connect
impl Connection for MockConnection {}

struct MockNetwork;

impl Connect for MockNetwork {
    type Connection = MockConnection;
    type Error = Error;

    fn connect(&mut self, _remote: &str) -> Result<Self::Connection, Self::Error> {
        // For simplicity, connect always succeeds and returns a new mock connection.
        // In a real scenario, this would involve address parsing and state.
        Ok(MockConnection::new())
    }
}

#[test]
fn test_connect_and_close() {
    let mut network = MockNetwork;
    let conn = network.connect("mock://server").unwrap();
    assert!(conn.is_open);
    conn.close().unwrap();
}

#[test]
fn test_read_write() {
    let mut conn = MockConnection::new();
    let write_data = [1, 2, 3, 4];

    // Test write
    let bytes_written = conn.write(&write_data).unwrap();
    assert_eq!(bytes_written, write_data.len());
    assert_eq!(&conn.write_buffer[..write_data.len()], &write_data);

    // Test read (after injecting data into the read buffer)
    let read_data = [5, 6, 7, 8];
    conn.set_read_data(&read_data);
    let mut read_buf = [0; 4];
    let bytes_read = conn.read(&mut read_buf).unwrap();
    assert_eq!(bytes_read, read_data.len());
    assert_eq!(read_buf, read_data);
}

#[test]
fn test_read_empty() {
    let mut conn = MockConnection::new();
    let mut read_buf = [0; 4];
    let bytes_read = conn.read(&mut read_buf).unwrap();
    assert_eq!(bytes_read, 0);
}

#[test]
fn test_write_full() {
    let mut conn = MockConnection::new();
    let large_data = [0xAA; MOCK_BUFFER_SIZE + 1];
    let bytes_written = conn.write(&large_data).unwrap();
    // Should only write up to the buffer size
    assert_eq!(bytes_written, MOCK_BUFFER_SIZE);
}

#[test]
fn test_op_on_closed_connection() {
    let mut conn = MockConnection::new();
    conn.is_open = false; // Manually set for test purposes.

    let mut buf = [0; 4];
    assert_eq!(conn.read(&mut buf), Err(Error::NotOpen));
    assert_eq!(conn.write(&[1, 2]), Err(Error::NotOpen));
    assert_eq!(conn.flush(), Err(Error::NotOpen));
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use futures::executor::block_on;

    // We need a separate Mock for async because of the `close` method signature difference.
    // However, for this implementation, we can reuse much of the logic.
    // Let's implement the async traits for the existing MockConnection.

    impl AsyncRead for MockConnection {
        type Error = Error;
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            // The synchronous implementation is already non-blocking.
            Read::read(self, buf)
        }
    }

    impl AsyncWrite for MockConnection {
        type Error = Error;
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            Write::write(self, buf)
        }
        async fn flush(&mut self) -> Result<(), Self::Error> {
            Write::flush(self)
        }
    }

    // The async close trait also consumes self.
    impl AsyncClose for MockConnection {
        type Error = Error;
        async fn close(self) -> Result<(), Self::Error> {
            // The sync implementation is fine.
            Close::close(self)
        }
    }

    impl AsyncConnection for MockConnection {}

    struct AsyncMockNetwork;

    impl AsyncConnect for AsyncMockNetwork {
        type Connection = MockConnection;
        type Error = Error;

        async fn connect(&mut self, _remote: &str) -> Result<Self::Connection, Self::Error> {
            Ok(MockConnection::new())
        }
    }

    #[test]
    fn test_async_read_write() {
        block_on(async {
            let mut network = AsyncMockNetwork;
            let mut conn = network.connect("mock://server").await.unwrap();

            let write_data = [10, 20, 30, 40];
            let bytes_written = conn.write(&write_data).await.unwrap();
            assert_eq!(bytes_written, write_data.len());

            // Since our mock isn't a real network, we have to manually
            // move the written data to the read buffer for testing.
            let mut temp_buf = [0; MOCK_BUFFER_SIZE];
            temp_buf[..bytes_written].copy_from_slice(&conn.write_buffer[..bytes_written]);
            conn.set_read_data(&temp_buf[..bytes_written]);

            let mut read_buf = [0; 4];
            let bytes_read = conn.read(&mut read_buf).await.unwrap();
            assert_eq!(bytes_read, write_data.len());
            assert_eq!(read_buf, write_data);

            conn.close().await.unwrap();
        });
    }
}
