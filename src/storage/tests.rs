use super::error::Error;
use super::*;

const MOCK_CAPACITY: usize = 1024;
const ERASED_BYTE: u8 = 0xFF;

struct MockStorage {
    memory: [u8; MOCK_CAPACITY],
    // For BlockStorage
    block_size: usize,
    // For SectorStorage
    sector_size: usize,
    // For UnifiedStorage
    non_volatile: bool,
}

impl MockStorage {
    fn new() -> Self {
        Self {
            memory: [ERASED_BYTE; MOCK_CAPACITY],
            block_size: 64,
            sector_size: 128,
            non_volatile: true,
        }
    }
}

impl ReadStorage for MockStorage {
    type Error = Error;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let offset = offset as usize;
        if offset + bytes.len() > self.memory.len() {
            return Err(Error::OutOfBounds);
        }
        bytes.copy_from_slice(&self.memory[offset..offset + bytes.len()]);
        Ok(())
    }

    fn capacity(&self) -> usize {
        MOCK_CAPACITY
    }
}

impl Storage for MockStorage {
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let offset = offset as usize;
        if offset + bytes.len() > self.memory.len() {
            return Err(Error::OutOfBounds);
        }
        self.memory[offset..offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
}

impl BlockingErase for MockStorage {
    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let from = from as usize;
        let to = to as usize;
        if to > self.memory.len() || from > to {
            return Err(Error::OutOfBounds);
        }
        for byte in &mut self.memory[from..to] {
            *byte = ERASED_BYTE;
        }
        Ok(())
    }
}

impl BlockStorage for MockStorage {
    fn block_size(&self) -> usize {
        self.block_size
    }
    fn block_count(&self) -> usize {
        ReadStorage::capacity(self) / self.block_size()
    }
}

impl SectorStorage for MockStorage {
    fn sector_size(&self) -> usize {
        self.sector_size
    }
    fn sector_count(&self) -> usize {
        ReadStorage::capacity(self) / self.sector_size()
    }
}

impl UnifiedStorage for MockStorage {
    fn is_non_volatile(&self) -> bool {
        self.non_volatile
    }
}

#[test]
fn test_read_write_erase() {
    let mut storage = MockStorage::new();
    let data = [0xDE, 0xAD, 0xBE, 0xEF];

    // Test write
    Storage::write(&mut storage, 0, &data).unwrap();

    // Test read
    let mut buf = [0; 4];
    ReadStorage::read(&mut storage, 0, &mut buf).unwrap();
    assert_eq!(buf, data);

    // Test erase
    BlockingErase::erase(&mut storage, 0, 4).unwrap();
    ReadStorage::read(&mut storage, 0, &mut buf).unwrap();
    assert_eq!(buf, [ERASED_BYTE; 4]);
}

#[test]
fn test_out_of_bounds() {
    let mut storage = MockStorage::new();
    let data = [0; 1];

    assert_eq!(
        Storage::write(&mut storage, MOCK_CAPACITY as u32, &data),
        Err(Error::OutOfBounds)
    );
    assert_eq!(
        ReadStorage::read(&mut storage, MOCK_CAPACITY as u32, &mut [0; 1]),
        Err(Error::OutOfBounds)
    );
    assert_eq!(
        BlockingErase::erase(&mut storage, 0, (MOCK_CAPACITY + 1) as u32),
        Err(Error::OutOfBounds)
    );
}

#[test]
fn test_block_and_sector() {
    let storage = MockStorage::new();
    assert_eq!(storage.block_size(), 64);
    assert_eq!(storage.block_count(), MOCK_CAPACITY / 64);
    assert_eq!(storage.sector_size(), 128);
    assert_eq!(storage.sector_count(), MOCK_CAPACITY / 128);
}

#[test]
fn test_unified_storage() {
    let storage = MockStorage::new();
    assert!(storage.is_non_volatile());
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use futures::executor::block_on;

    impl AsyncReadStorage for MockStorage {
        type Error = Error;

        async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
            ReadStorage::read(self, offset, bytes)
        }

        fn capacity(&self) -> usize {
            ReadStorage::capacity(self)
        }
    }

    impl AsyncStorage for MockStorage {
        async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
            Storage::write(self, offset, bytes)
        }
    }

    impl AsyncErase for MockStorage {
        async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
            BlockingErase::erase(self, from, to)
        }
    }

    #[test]
    fn test_async_read_write_erase() {
        block_on(async {
            let mut storage = MockStorage::new();
            let data = [0xAB, 0xCD, 0xEF, 0x12];

            AsyncStorage::write(&mut storage, 10, &data).await.unwrap();

            let mut buf = [0; 4];
            AsyncReadStorage::read(&mut storage, 10, &mut buf)
                .await
                .unwrap();
            assert_eq!(buf, data);

            AsyncErase::erase(&mut storage, 10, 14).await.unwrap();
            AsyncReadStorage::read(&mut storage, 10, &mut buf)
                .await
                .unwrap();
            assert_eq!(buf, [ERASED_BYTE; 4]);
        });
    }
}
