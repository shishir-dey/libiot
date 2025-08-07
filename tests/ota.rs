use libiot::ota::*;
use libiot::network::{Connection, Read, Write, Close};
use heapless::String;
use core::str::FromStr;

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

#[derive(Default)]
struct MockPlatform {
    firmware_saved: bool,
    firmware_activated: bool,
}

impl Platform for MockPlatform {
    fn save_firmware(&mut self, _firmware: &[u8]) -> Result<(), Error> {
        self.firmware_saved = true;
        Ok(())
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

    let firmware = Firmware {
        version: 1,
        size: 1024,
        url: String::<256>::from_str("http://example.com/firmware").unwrap(),
    };

    // Initial state should be Idle
    assert_eq!(*agent.state(), State::Idle);

    agent.process_event(Event::UpdateAvailable(firmware)).unwrap();
    assert_eq!(*agent.state(), State::Downloading);
    // In a real implementation, the `download_firmware` method would be called here.
    // Since it's a placeholder, we can't test much about it.

    agent.process_event(Event::DownloadComplete).unwrap();
    assert_eq!(*agent.state(), State::Verifying);
    // In a real implementation, the `verify_firmware` method would be called here.

    agent.process_event(Event::VerificationComplete).unwrap();
    assert_eq!(*agent.state(), State::Activating);
    assert!(agent.platform().firmware_activated);

    agent.process_event(Event::ActivationComplete).unwrap();
    assert_eq!(*agent.state(), State::Idle);
}
