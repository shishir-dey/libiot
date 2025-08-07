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
    partitions: [Partition; 2],
    active_partition_index: usize,
    boot_partition_index: usize,
    ota_data: OtaData,
    firmware: [u8; 8192],
}

impl Default for MockPlatform {
    fn default() -> Self {
        Self {
            partitions: [
                Partition { start: 0, size: 4096 },
                Partition { start: 4096, size: 4096 },
            ],
            active_partition_index: 0,
            boot_partition_index: 0,
            ota_data: OtaData {
                state: OtaState::Idle,
                version: 0,
                checksum: 0,
            },
            firmware: [0u8; 8192],
        }
    }
}

impl Platform for MockPlatform {
    fn get_active_partition(&self) -> Partition {
        self.partitions[self.active_partition_index]
    }

    fn get_inactive_partition(&self) -> Partition {
        self.partitions[1 - self.active_partition_index]
    }

    fn set_boot_partition(&mut self, partition: Partition) -> Result<(), Error> {
        if partition == self.partitions[0] {
            self.boot_partition_index = 0;
        } else if partition == self.partitions[1] {
            self.boot_partition_index = 1;
        } else {
            return Err(Error::PlatformError);
        }
        Ok(())
    }

    fn get_ota_data(&self) -> Result<OtaData, Error> {
        Ok(self.ota_data.clone())
    }

    fn set_ota_data(&mut self, data: &OtaData) -> Result<(), Error> {
        self.ota_data = data.clone();
        Ok(())
    }

    fn save_firmware_chunk(&mut self, offset: u32, chunk: &[u8]) -> Result<(), Error> {
        let inactive_partition = self.get_inactive_partition();
        let start = inactive_partition.start as usize;
        let offset = offset as usize;
        let new_len = offset + chunk.len();
        if new_len > inactive_partition.size as usize {
            return Err(Error::DownloadError);
        }
        self.firmware[start + offset..start + new_len].copy_from_slice(chunk);
        Ok(())
    }

    fn read_firmware_chunk(&self, offset: u32, length: u32) -> Result<&[u8], Error> {
        let inactive_partition = self.get_inactive_partition();
        let start = inactive_partition.start as usize;
        let offset = offset as usize;
        let length = length as usize;
        if offset + length > inactive_partition.size as usize {
            return Err(Error::VerificationError);
        }
        Ok(&self.firmware[start + offset..start + offset + length])
    }

    fn activate_firmware(&mut self) -> Result<(), Error> {
        self.active_partition_index = self.boot_partition_index;
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
    agent.platform.save_firmware_chunk(0, &firmware_data).unwrap();

    agent.process_event(Event::DownloadComplete).unwrap();
    assert_eq!(*agent.state(), State::Verifying(firmware.clone()));

    agent.process_event(Event::VerificationComplete).unwrap();
    assert_eq!(*agent.state(), State::Activating);
    assert!(agent.platform.get_ota_data().unwrap().state == OtaState::Pending);

    agent.process_event(Event::ActivationComplete).unwrap();
    assert_eq!(*agent.state(), State::Idle);
}

#[test]
fn test_ota_ab_flow() {
    let platform = MockPlatform::default();
    let connection = MockConnection;
    let mut ota_manager = OtaManager::new(connection, platform);

    // Simulate a successful update
    let mut firmware = ota_manager.agent.request_update().unwrap().unwrap();
    let mut hasher = Hasher::new();
    let mut firmware_data = [0u8; 1024];
    for i in 0..1024 {
        firmware_data[i] = i as u8;
    }
    hasher.update(&firmware_data);
    firmware.checksum = hasher.finalize();

    ota_manager.agent.process_event(Event::UpdateAvailable(firmware.clone())).unwrap();
    ota_manager.agent.platform.save_firmware_chunk(0, &firmware_data).unwrap();
    ota_manager.agent.process_event(Event::DownloadComplete).unwrap();
    ota_manager.agent.process_event(Event::VerificationComplete).unwrap();
    ota_manager.agent.process_event(Event::ActivationComplete).unwrap();

    // After reboot, the OtaManager should be created again.
    // The `new` function should handle the post-reboot logic.
    let connection2 = MockConnection;
    let mut ota_manager2 = OtaManager::new(connection2, ota_manager.agent.platform);

    assert_eq!(ota_manager2.agent.platform.get_ota_data().unwrap().state, OtaState::Success);
    assert_eq!(ota_manager2.agent.platform.get_active_partition(), ota_manager2.agent.platform.partitions[1]);
}
