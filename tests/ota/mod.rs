use libiot::network::application::http::client::Client as HttpClient;
use libiot::network::{Close, Connection, Read, Write};
use libiot::ota::{Config, HttpSource, Ota};
use libiot::storage::{BlockingErase, Storage};

// -------------------------
// RAM-based Storage Mock
// -------------------------

#[derive(Debug)]
struct RamStorage<const N: usize> {
    buf: [u8; N],
}

impl<const N: usize> RamStorage<N> {
    fn new() -> Self {
        Self { buf: [0xFF; N] }
    }
}

impl<const N: usize> libiot::storage::ReadStorage for RamStorage<N> {
    type Error = libiot::storage::error::Error;
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let off = offset as usize;
        if off + bytes.len() > self.buf.len() {
            return Err(libiot::storage::error::Error::OutOfBounds);
        }
        bytes.copy_from_slice(&self.buf[off..off + bytes.len()]);
        Ok(())
    }
    fn capacity(&self) -> usize {
        self.buf.len()
    }
}

impl<const N: usize> Storage for RamStorage<N> {
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let off = offset as usize;
        if off + bytes.len() > self.buf.len() {
            return Err(libiot::storage::error::Error::OutOfBounds);
        }
        self.buf[off..off + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
}

impl<const N: usize> BlockingErase for RamStorage<N> {
    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let f = from as usize;
        let t = to as usize;
        if f > t || t > self.buf.len() {
            return Err(libiot::storage::error::Error::OutOfBounds);
        }
        for b in &mut self.buf[f..t] {
            *b = 0xFF;
        }
        Ok(())
    }
}

// --------------------------------
// Chaos Connection (jittery link)
// --------------------------------

#[derive(Debug)]
struct ChaosConnection {
    incoming: std::vec::Vec<u8>,
    written: std::vec::Vec<u8>,
    object: std::vec::Vec<u8>,
    drop_every: usize,
    partial_max: usize,
    read_count: usize,
    delivered_in_phase: usize,
}

impl ChaosConnection {
    fn new(object: &[u8], drop_every: usize, partial_max: usize) -> Self {
        Self {
            incoming: std::vec::Vec::new(),
            written: std::vec::Vec::new(),
            object: object.to_vec(),
            drop_every,
            partial_max,
            read_count: 0,
            delivered_in_phase: 0,
        }
    }

    fn prepare_response_for_request(&mut self) {
        if self.written.is_empty() {
            return;
        }
        // Parse Range header if present anywhere in the written buffer
        let text = String::from_utf8_lossy(&self.written);
        let mut range: Option<(usize, usize)> = None;
        for line in text.lines() {
            if let Some(idx) = line.to_ascii_lowercase().find("range:") {
                let value = line[idx + 6..].trim();
                if let Some(eq_idx) = value.find('=') {
                    let spec = &value[eq_idx + 1..];
                    if let Some(dash) = spec.find('-') {
                        let start_str = &spec[..dash];
                        let end_str = &spec[dash + 1..];
                        if let (Ok(start), Ok(end)) = (
                            start_str.trim().parse::<usize>(),
                            end_str.trim().parse::<usize>(),
                        ) {
                            if start <= end && end < self.object.len() {
                                range = Some((start, end));
                                break;
                            }
                        }
                    }
                }
            }
        }
        let (status, body, content_range) = if let Some((s, e)) = range {
            (206u16, &self.object[s..=e], Some((s, e, self.object.len())))
        } else {
            (200u16, &self.object[..], None)
        };
        let resp = build_http_response(status, body, true, content_range);
        self.incoming.extend_from_slice(&resp);
        self.written.clear();
        self.delivered_in_phase = 0;
    }
}

impl Read for ChaosConnection {
    type Error = libiot::network::error::Error;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read_count += 1;
        if self.incoming.is_empty() {
            return Ok(0);
        }
        let mut n = core::cmp::min(
            buf.len(),
            core::cmp::min(self.incoming.len(), self.partial_max.max(1)),
        );
        // Simulate jitter by occasionally limiting to 1 byte instead of returning 0
        if self.drop_every > 0
            && self.read_count % self.drop_every == 0
            && self.delivered_in_phase > 0
        {
            n = core::cmp::min(n, 1);
        }
        let data: std::vec::Vec<u8> = self.incoming.drain(..n).collect();
        buf[..n].copy_from_slice(&data);
        self.delivered_in_phase += n;
        Ok(n)
    }
}

impl Write for ChaosConnection {
    type Error = libiot::network::error::Error;
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.written.extend_from_slice(buf);
        self.prepare_response_for_request();
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Close for ChaosConnection {
    type Error = libiot::network::error::Error;
    fn close(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Connection for ChaosConnection {}

fn build_http_response(
    status: u16,
    body: &[u8],
    content_length: bool,
    content_range: Option<(usize, usize, usize)>,
) -> std::vec::Vec<u8> {
    let mut out = std::vec::Vec::new();
    match status {
        200 => out.extend_from_slice(b"HTTP/1.1 200 OK\r\n"),
        206 => out.extend_from_slice(b"HTTP/1.1 206 Partial Content\r\n"),
        _ => out.extend_from_slice(b"HTTP/1.1 500 Error\r\n"),
    };
    if let Some((s, e, total)) = content_range {
        out.extend_from_slice(format!("Content-Range: bytes {}-{}/{}\r\n", s, e, total).as_bytes());
    }
    if content_length {
        out.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
    out
}

#[test]
fn ota_http_download_with_jittery_network() {
    let total_size = 8 * 1024;
    let mut firmware = std::vec::Vec::with_capacity(total_size);
    for i in 0..total_size {
        firmware.push((i % 251) as u8);
    }

    let chaos = ChaosConnection::new(&firmware, 5, 97);
    let mut http = HttpClient::new(chaos);

    let mut storage = RamStorage::<{ 16 * 1024 }>::new();

    let cfg = Config {
        chunk_size: 1024,
        erase_before_write: true,
        verify_crc32: false,
    };
    let mut ota = Ota::new(cfg).unwrap();

    let src = HttpSource {
        host: "example.com",
        path: "/fw.bin",
        size: firmware.len(),
        crc32: None,
    };
    ota.run_http(
        &mut http,
        &mut storage,
        0,
        &src,
        None::<&mut libiot::ota::MqttProgress<'_, ChaosConnection>>,
    )
    .unwrap();

    let mut read_back = vec![0u8; firmware.len()];
    libiot::storage::ReadStorage::read(&mut storage, 0, &mut read_back).unwrap();
    assert_eq!(read_back, firmware);
}

#[test]
fn ota_http_download_large_hex_like_payload_with_jitter() {
    // Generate a large binary payload (~32 KiB) to simulate real-world size
    let total = 32 * 1024usize;
    let mut body_bytes = vec![0u8; total];
    for i in 0..total {
        body_bytes[i] = (i % 251) as u8;
    }

    let chaos = ChaosConnection::new(&body_bytes, 7, 113);
    let mut http = HttpClient::new(chaos);

    // Allocate storage sized comfortably above payload size to avoid stack overflow
    let mut storage = Box::new(RamStorage::<{ 128 * 1024 }>::new());

    let cfg = Config {
        chunk_size: 1024,
        erase_before_write: true,
        verify_crc32: false,
    };
    let mut ota = Ota::new(cfg).unwrap();
    let src = HttpSource {
        host: "micropython.org",
        path: "/resources/firmware/STM32F4DISC-20250415-v1.25.0.hex",
        size: body_bytes.len(),
        crc32: None,
    };

    ota.run_http(
        &mut http,
        &mut *storage,
        0,
        &src,
        None::<&mut libiot::ota::MqttProgress<'_, ChaosConnection>>,
    )
    .unwrap();

    let mut read_back = vec![0u8; body_bytes.len()];
    libiot::storage::ReadStorage::read(&mut *storage, 0, &mut read_back).unwrap();
    assert_eq!(read_back.len(), body_bytes.len());
    assert_eq!(&read_back[..256], &body_bytes[..256]);
    assert_eq!(
        &read_back[read_back.len() - 256..],
        &body_bytes[body_bytes.len() - 256..]
    );
}
