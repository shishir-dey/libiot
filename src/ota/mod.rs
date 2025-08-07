#![allow(missing_docs)]
#![allow(async_fn_in_trait)]
#![deny(unsafe_code)]

//! # Over-the-Air (OTA) Update Agent
//!
//! This module provides a network-agnostic OTA update mechanism inspired by the
//! AWS IoT Core OTA service. It is designed to be portable and work with any
//! network stack that implements the `libiot::network` traits.
//!
//! ## Design
//!
//! The OTA agent is designed as a state machine that is driven by events. The
//! agent can be in one of the following states:
//!
//! * `Idle`: The agent is waiting for an update.
//! * `Downloading`: The agent is downloading the firmware image.
//! * `Verifying`: The agent is verifying the integrity of the firmware image.
//! * `Activating`: The agent is activating the new firmware image.
//!
//! The agent transitions between states based on events that it receives. The
//! following events are supported:
//!
//! * `UpdateAvailable`: A new firmware update is available.
//! * `DownloadComplete`: The firmware image has been successfully downloaded.
//! * `VerificationComplete`: The firmware image has been successfully verified.
//! * `ActivationComplete`: The new firmware image has been successfully activated.
//!
//! The OTA agent is designed to be used with a `Platform` trait that provides
//! platform-specific functionality, such as saving the firmware image to flash
//! storage and rebooting the device.
use crate::network::Connection;
use crate::storage::Region;
use heapless::String;
use core::str::FromStr;
use base64ct::{Base64, Encoding as B64Encoding};

/// Represents a partition on a storage device.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Partition {
    /// The start address of the partition.
    pub start: u32,
    /// The size of the partition in bytes.
    pub size: u32,
}

impl Region for Partition {
    fn start(&self) -> u32 {
        self.start
    }

    fn end(&self) -> u32 {
        self.start + self.size
    }
}

/// Represents the state of the OTA update.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OtaState {
    /// No OTA update is in progress.
    Idle,
    /// An OTA update is pending and will be applied on the next reboot.
    Pending,
    /// The OTA update was successful.
    Success,
    /// The OTA update failed.
    Failed,
}

/// Represents the OTA metadata that is stored in a non-volatile storage.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OtaData {
    /// The state of the OTA update.
    pub state: OtaState,
    /// The version of the firmware in the inactive partition.
    pub version: u32,
    /// The checksum of the firmware in the inactive partition.
    pub checksum: u32,
}

/// Represents the state of the OTA agent.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// The agent is idle and waiting for an update.
    Idle,
    /// The agent is downloading the firmware image.
    Downloading(Firmware),
    /// The agent is verifying the integrity of the firmware image.
    Verifying(Firmware),
    /// The agent is activating the new firmware image.
    Activating,
}

/// Represents an event that can trigger a state transition in the OTA agent.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// A new firmware update is available.
    UpdateAvailable(Firmware),
    /// The firmware image has been successfully downloaded.
    DownloadComplete,
    /// The firmware image has been successfully verified.
    VerificationComplete,
    /// The new firmware image has been successfully activated.
    ActivationComplete,
}

/// Represents an error that can occur during the OTA update process.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// An error occurred while downloading the firmware image.
    DownloadError,
    /// An error occurred while verifying the firmware image.
    VerificationError,
    /// An error occurred while activating the new firmware image.
    ActivationError,
    /// A platform-specific error occurred.
    PlatformError,
}

/// The encoding of the firmware image.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Encoding {
    /// Raw binary.
    Raw,
    /// Base64-encoded.
    Base64,
}

/// Represents a firmware image.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Firmware {
    /// The version of the firmware image.
    pub version: u32,
    /// The size of the firmware image in bytes.
    pub size: u32,
    /// The URL from which the firmware image can be downloaded.
    pub url: String<256>,
    /// The encoding of the firmware image.
    pub encoding: Encoding,
    /// The checksum of the firmware image.
    pub checksum: u32,
}

/// A trait for platform-specific OTA functionality.
///
/// This trait must be implemented by the target platform to provide the
/// necessary functionality for the OTA agent to work.
pub trait Platform {
    /// Gets the active partition.
    fn get_active_partition(&self) -> Partition;

    /// Gets the inactive partition.
    fn get_inactive_partition(&self) -> Partition;

    /// Sets the partition that should be booted on the next reboot.
    fn set_boot_partition(&mut self, partition: Partition) -> Result<(), Error>;

    /// Gets the OTA metadata.
    fn get_ota_data(&self) -> Result<OtaData, Error>;

    /// Sets the OTA metadata.
    fn set_ota_data(&mut self, data: &OtaData) -> Result<(), Error>;

    /// Saves a chunk of the firmware image to the inactive partition.
    fn save_firmware_chunk(&mut self, offset: u32, chunk: &[u8]) -> Result<(), Error>;

    /// Reads a chunk of the firmware image from the inactive partition.
    fn read_firmware_chunk(&self, offset: u32, length: u32) -> Result<&[u8], Error>;

    /// Activates the new firmware image.
    ///
    /// This function should reboot the device and boot into the new firmware
    /// image.
    fn activate_firmware(&mut self) -> Result<(), Error>;
}

/// The OTA agent.
///
/// This struct manages the state of the OTA update process.
pub struct OtaAgent<P: Platform> {
    /// The current state of the OTA agent.
    state: State,
    /// The platform-specific functionality.
    pub platform: P,
}

impl<P: Platform> OtaAgent<P> {
    /// Returns the current state of the OTA agent.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Creates a new OTA agent.
    pub fn new(platform: P) -> Self {
        Self {
            state: State::Idle,
            platform,
        }
    }

    /// Processes an OTA event and transitions the state machine.
    pub fn process_event(&mut self, event: Event) -> Result<(), Error> {
        match self.state.clone() {
            State::Idle => {
                if let Event::UpdateAvailable(firmware) = event {
                    self.download_firmware(&firmware)?;
                    self.state = State::Downloading(firmware);
                }
            }
            State::Downloading(firmware) => {
                if let Event::DownloadComplete = event {
                    self.verify_firmware(firmware.size, firmware.checksum)?;
                    self.state = State::Verifying(firmware);
                }
            }
            State::Verifying(_firmware) => {
                if let Event::VerificationComplete = event {
                    self.activate_firmware()?;
                    self.state = State::Activating;
                }
            }
            State::Activating => {
                if let Event::ActivationComplete = event {
                    self.state = State::Idle;
                }
            }
        }
        Ok(())
    }

    /// Requests an update from the server.
    pub fn request_update(&mut self) -> Result<Option<Firmware>, Error> {
        // In a real implementation, we would send a request to the update server
        // and parse the response. For now, we will just simulate a response.
        let firmware = Firmware {
            version: 2,
            size: 1024,
            url: String::<256>::from_str("http://example.com/firmware_v2").unwrap(),
            encoding: Encoding::Raw,
            checksum: 0, // In a real implementation, this would come from the server
        };
        Ok(Some(firmware))
    }

    /// Downloads the firmware image from the server.
    fn download_firmware(&mut self, firmware: &Firmware) -> Result<(), Error> {
        let _inactive_partition = self.platform.get_inactive_partition();
        let mut buffer = [0u8; 1024];
        let mut bytes_downloaded = 0;

        // In a real implementation, we would use the URL from the firmware struct
        // to establish a connection to the download server. For now, we assume
        // that the connection is already established.

        while bytes_downloaded < firmware.size {
            // let bytes_to_read = core::cmp::min(buffer.len() as u32, firmware.size - bytes_downloaded);
            // In a real implementation, we would read from the network here.
            // For now, we will just fill the buffer with dummy data.
            let bytes_read = 1024; // Simulate reading 1024 bytes
            for i in 0..bytes_read {
                buffer[i] = (bytes_downloaded + i as u32) as u8;
            }

            match firmware.encoding {
                Encoding::Raw => {
                    self.platform.save_firmware_chunk(bytes_downloaded, &buffer[..bytes_read])?;
                }
                Encoding::Base64 => {
                    let mut decoded_buffer = [0u8; 1024];
                    let decoded_len = Base64::decode(&buffer[..bytes_read], &mut decoded_buffer)
                        .map_err(|_| Error::DownloadError)?
                        .len();
                    self.platform.save_firmware_chunk(bytes_downloaded, &decoded_buffer[..decoded_len])?;
                }
            }
            bytes_downloaded += bytes_read as u32;
        }

        let ota_data = OtaData {
            state: OtaState::Pending,
            version: firmware.version,
            checksum: firmware.checksum,
        };
        self.platform.set_ota_data(&ota_data)?;

        Ok(())
    }

    /// Verifies the integrity of the firmware image.
    fn verify_firmware(&mut self, firmware_size: u32, expected_checksum: u32) -> Result<(), Error> {
        let _inactive_partition = self.platform.get_inactive_partition();
        let mut hasher = crc32fast::Hasher::new();
        let mut bytes_verified = 0;

        while bytes_verified < firmware_size {
            let chunk = self.platform.read_firmware_chunk(bytes_verified, 1024)?;
            hasher.update(chunk);
            bytes_verified += chunk.len() as u32;
        }

        let checksum = hasher.finalize();
        if checksum == expected_checksum {
            Ok(())
        } else {
            let ota_data = OtaData {
                state: OtaState::Failed,
                version: 0,
                checksum: 0,
            };
            self.platform.set_ota_data(&ota_data)?;
            Err(Error::VerificationError)
        }
    }

    /// Activates the new firmware image.
    fn activate_firmware(&mut self) -> Result<(), Error> {
        let inactive_partition = self.platform.get_inactive_partition();
        self.platform.set_boot_partition(inactive_partition)?;
        self.platform.activate_firmware()
    }
}

/// The OTA manager.
///
/// This struct manages the OTA update process for a specific network connection.
pub struct OtaManager<C: Connection, P: Platform> {
    /// The network connection to the update server.
    pub connection: C,
    /// The OTA agent.
    pub agent: OtaAgent<P>,
}

impl<C: Connection, P: Platform> OtaManager<C, P> {
    /// Creates a new OTA manager.
    pub fn new(connection: C, mut platform: P) -> Self {
        if let Ok(mut ota_data) = platform.get_ota_data() {
            if ota_data.state == OtaState::Pending {
                // The new firmware has been booted. We can now mark the update
                // as successful. In a real application, we would run some
                // self-tests here to verify the new firmware.
                ota_data.state = OtaState::Success;
                platform.set_ota_data(&ota_data).unwrap();
            }
        }

        Self {
            connection,
            agent: OtaAgent::new(platform),
        }
    }

    /// Runs the OTA manager.
    ///
    /// This function listens for update notifications and drives the OTA agent.
    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            if let Some(firmware) = self.agent.request_update()? {
                self.agent.process_event(Event::UpdateAvailable(firmware))?;
                self.agent.process_event(Event::DownloadComplete)?;
                self.agent.process_event(Event::VerificationComplete)?;
                self.agent.process_event(Event::ActivationComplete)?;
            }
            // In a real implementation, we would have a delay here.
        }
    }
}
