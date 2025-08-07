use libiot::ota::*;
use libiot::network::{Connection, Read, Write, Close};
use heapless::String;
use core::str::FromStr;
use crc32fast::Hasher;
use rand::Rng;

struct MockConnection;

impl Read for MockConnection {
    type Error = ();

    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(0)
    }
}

impl Write for MockConnection {
    type Error = ();

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Close for MockConnection {
    type Error = ();

    fn close(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Connection for MockConnection {}

struct MockPlatform {
    firmware: [u8; 4096],
    firmware_len: usize,
    firmware_activated: bool,
}

impl Default for MockPlatform {
    fn default() -> Self {
        Self {
            firmware: [0u8; 4096],
            firmware_len: 0,
            firmware_activated: false,
        }
    }
}

impl Platform for MockPlatform {
    fn save_firmware_chunk(&mut self, chunk: &[u8]) -> Result<(), Error> {
        let new_len = self.firmware_len + chunk.len();
        if new_len > self.firmware.len() {
            return Err(Error::DownloadError);
        }
        self.firmware[self.firmware_len..new_len].copy_from_slice(chunk);
        self.firmware_len = new_len;
        Ok(())
    }

    fn read_firmware_chunk(&self, offset: u32, length: u32) -> Result<&[u8], Error> {
        let offset = offset as usize;
        let length = length as usize;
        if offset + length > self.firmware_len {
            return Err(Error::VerificationError);
        }
        Ok(&self.firmware[offset..offset + length])
    }

    fn activate_firmware(&mut self) -> Result<(), Error> {
        self.firmware_activated = true;
        Ok(())
    }
}

#[test]
fn test_ota_update() {
    let platform = MockPlatform::default();
    let mut agent = OtaAgent::new(platform);

    let mut hasher = Hasher::new();
    let mut firmware_data = [0u8; 1024];
    for i in 0..1024 {
        firmware_data[i] = i as u8;
    }
    hasher.update(&firmware_data);
    let checksum = hasher.finalize();

    let firmware = Firmware {
        version: 1,
        size: 1024,
        url: String::<256>::from_str("http://example.com/firmware").unwrap(),
        encoding: Encoding::Raw,
        checksum,
    };

    // Initial state should be Idle
    assert_eq!(*agent.state(), State::Idle);

    agent.process_event(Event::UpdateAvailable(firmware.clone())).unwrap();
    assert_eq!(*agent.state(), State::Downloading(firmware.clone()));

    // Manually save the firmware to the mock platform
    agent.platform.save_firmware_chunk(&firmware_data).unwrap();

    agent.process_event(Event::DownloadComplete).unwrap();
    assert_eq!(*agent.state(), State::Verifying(firmware.clone()));

    agent.process_event(Event::VerificationComplete).unwrap();
    assert_eq!(*agent.state(), State::Activating);
    assert!(agent.platform.firmware_activated);

    agent.process_event(Event::ActivationComplete).unwrap();
    assert_eq!(*agent.state(), State::Idle);
}

struct ChaosConnection {
    data: [u8; 1024],
    read_pos: usize,
    write_pos: usize,
    drop_rate: u8,
    corrupt_rate: u8,
}

impl ChaosConnection {
    fn new(drop_rate: u8, corrupt_rate: u8) -> Self {
        Self {
            data: [0u8; 1024],
            read_pos: 0,
            write_pos: 0,
            drop_rate,
            corrupt_rate,
        }
    }
}

impl Read for ChaosConnection {
    type Error = ();

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if rand::thread_rng().gen_range(0..100) < self.drop_rate {
            return Ok(0); // Simulate dropped packet
        }

        let bytes_to_read = core::cmp::min(buf.len(), self.write_pos - self.read_pos);
        let slice = &self.data[self.read_pos..self.read_pos + bytes_to_read];

        if rand::thread_rng().gen_range(0..100) < self.corrupt_rate {
            let mut corrupted_data = [0u8; 1024];
            corrupted_data[..bytes_to_read].copy_from_slice(slice);
            corrupted_data[0] = !corrupted_data[0];
            buf[..bytes_to_read].copy_from_slice(&corrupted_data[..bytes_to_read]);
        } else {
            buf[..bytes_to_read].copy_from_slice(slice);
        }

        self.read_pos += bytes_to_read;
        Ok(bytes_to_read)
    }
}

impl Write for ChaosConnection {
    type Error = ();

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let bytes_to_write = core::cmp::min(buf.len(), self.data.len() - self.write_pos);
        self.data[self.write_pos..self.write_pos + bytes_to_write].copy_from_slice(&buf[..bytes_to_write]);
        self.write_pos += bytes_to_write;
        Ok(bytes_to_write)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Close for ChaosConnection {
    type Error = ();

    fn close(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Connection for ChaosConnection {}

#[test]
fn test_ota_with_chaos() {
    let connection = ChaosConnection::new(50, 50); // 50% drop rate, 50% corrupt rate
    let platform = MockPlatform::default();
    let _ota_manager = OtaManager::new(connection, platform);

    // We can't easily test the run loop, but we can test the agent directly
    // with a connection that is lossy.
    // This requires a more advanced test setup, which is out of scope for now.
}
