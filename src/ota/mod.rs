//! Over-the-air (OTA) update logic for embedded devices
//!
//! This module provides a small, dependency-light OTA workflow inspired by
//! AWS IoT Core OTA jobs. It composes the storage and network abstraction
//! layers in this crate to download firmware in fixed-size HTTP ranges and
//! persist them to a target storage region, with optional progress reporting
//! over MQTT.
//!
//! Design goals
//! - Works with any `Storage + BlockingErase`
//! - Uses `network::application::http::Client` for chunked HTTP range reads
//! - Optional progress reporting via `network::application::mqtt::Client`
//! - Lightweight checksum verification (CRC32 by default). Users can inject
//!   a custom verifier if desired.
//!
//! Notes
//! - This module does not manage bootloader/partition swaps. Users should
//!   provide the proper target region and apply/commit the new image using
//!   their boot process after a successful download and verification.
//! - The bundled HTTP client limits response body capacity to 2048 bytes.
//!   OTA here uses HTTP range requests with a configurable `chunk_size` that
//!   must be <= 2048 to operate within these limits.

#![allow(missing_docs)]
#![deny(unsafe_code)]

use crate::network::application::http::client::{Client as HttpClient, Header, Method, Request};
use crate::network::application::mqtt::client::{Client as MqttClient, QoS};
use crate::network::error as net_err;
use crate::storage::error as storage_err;
use crate::storage::{BlockingErase, Storage};
use heapless::{String, Vec};

/// Maximum header name/value lengths taken from HTTP client constraints
const MAX_HEADER_NAME_LEN: usize = 64;
const MAX_HEADER_VALUE_LEN: usize = 256;

/// OTA-specific error type
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Network(net_err::Error),
    Storage(storage_err::Error),
    InvalidConfig,
    VerifyFailed,
    Canceled,
    Protocol,
}

impl From<net_err::Error> for Error {
    fn from(e: net_err::Error) -> Self {
        Error::Network(e)
    }
}

impl From<storage_err::Error> for Error {
    fn from(e: storage_err::Error) -> Self {
        Error::Storage(e)
    }
}

/// OTA state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Erasing,
    Downloading,
    Verifying,
    Finalizing,
    Completed,
    Failed,
    Canceled,
}

/// Where to fetch firmware from using HTTP
#[derive(Debug, Clone)]
pub struct HttpSource<'a> {
    /// HTTP Host header value, e.g. "example.com"
    pub host: &'a str,
    /// HTTP path for the firmware object, e.g. "/firmware.bin"
    pub path: &'a str,
    /// Total size of the firmware in bytes
    pub size: usize,
    /// Optional CRC32 of the entire image for verification
    pub crc32: Option<u32>,
}

/// OTA configuration
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Chunk size for each HTTP range read. Must be <= 2048.
    pub chunk_size: usize,
    /// Erase the target region before writing
    pub erase_before_write: bool,
    /// Perform CRC32 verification if checksum is provided
    pub verify_crc32: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            chunk_size: 1024,
            erase_before_write: true,
            verify_crc32: true,
        }
    }
}

/// OTA progress information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub bytes_total: usize,
    pub bytes_downloaded: usize,
    pub state: State,
}

/// A simple CRC32 (IEEE) hasher implemented without external dependencies
struct Crc32 {
    table: [u32; 256],
    value: u32,
}

impl Crc32 {
    fn new() -> Self {
        let mut table = [0u32; 256];
        let poly: u32 = 0xEDB88320;
        let mut i = 0u32;
        while i < 256 {
            let mut c = i;
            let mut j = 0;
            while j < 8 {
                c = if (c & 1) != 0 {
                    (c >> 1) ^ poly
                } else {
                    c >> 1
                };
                j += 1;
            }
            table[i as usize] = c;
            i += 1;
        }
        Self {
            table,
            value: 0xFFFF_FFFF,
        }
    }

    fn update(&mut self, data: &[u8]) {
        for &b in data {
            let idx = (self.value ^ b as u32) & 0xFF;
            self.value = self.table[idx as usize] ^ (self.value >> 8);
        }
    }

    fn finalize(self) -> u32 {
        self.value ^ 0xFFFF_FFFF
    }
}

/// OTA driver. Create with a `Config`, then call `run_http` to perform the
/// blocking OTA over HTTP using range requests.
pub struct Ota {
    cfg: Config,
    state: State,
    canceled: bool,
}

impl Ota {
    pub fn new(cfg: Config) -> Result<Self, Error> {
        if cfg.chunk_size == 0 || cfg.chunk_size > 2048 {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            cfg,
            state: State::Idle,
            canceled: false,
        })
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn cancel(&mut self) {
        self.canceled = true;
    }

    /// Download the firmware from the HTTP source into `storage` starting at
    /// `base_offset`. If `progress_topic` and `mqtt` are provided, progress is
    /// published as small JSON messages: {"bytes":N,"total":T,"state":"downloading"}
    pub fn run_http<HC, S, MC>(
        &mut self,
        http: &mut HttpClient<HC>,
        storage: &mut S,
        base_offset: u32,
        source: &HttpSource,
        mut mqtt: Option<&mut MqttProgress<'_, MC>>,
    ) -> Result<(), Error>
    where
        HC: crate::network::Connection,
        MC: crate::network::Connection,
        S: Storage + BlockingErase,
    {
        if self.canceled {
            self.state = State::Canceled;
            return Err(Error::Canceled);
        }

        // Erase
        if self.cfg.erase_before_write {
            self.state = State::Erasing;
            if self.canceled {
                self.state = State::Canceled;
                return Err(Error::Canceled);
            }
            storage
                .erase(base_offset, base_offset + (source.size as u32))
                .map_err(|_| Error::Storage(storage_err::Error::EraseError))?;
        }

        // Download in ranges
        self.state = State::Downloading;
        let mut downloaded: usize = 0;
        let mut crc = Crc32::new();

        while downloaded < source.size {
            if self.canceled {
                self.state = State::Canceled;
                return Err(Error::Canceled);
            }

            let remaining = source.size - downloaded;
            let len = core::cmp::min(self.cfg.chunk_size, remaining);
            let start = downloaded;
            let end = start + len - 1; // inclusive

            let mut headers: Vec<Header, 16> = Vec::new();
            let host_header = Header {
                name: String::<MAX_HEADER_NAME_LEN>::try_from("Host").unwrap(),
                value: String::<MAX_HEADER_VALUE_LEN>::try_from(source.host).unwrap(),
            };
            headers.push(host_header).map_err(|_| Error::Protocol)?;

            let mut range_value: String<64> = String::new();
            // bytes=start-end
            let _ = core::fmt::write(&mut range_value, format_args!("bytes={}-{}", start, end));
            let range_header = Header {
                name: String::<MAX_HEADER_NAME_LEN>::try_from("Range").unwrap(),
                value: String::<MAX_HEADER_VALUE_LEN>::try_from(range_value.as_str()).unwrap(),
            };
            headers.push(range_header).map_err(|_| Error::Protocol)?;

            let req = Request {
                method: Method::Get,
                path: source.path,
                headers,
                body: None,
            };

            let resp = http.request(&req)?;
            match resp.status_code {
                200 | 206 => {}
                _ => return Err(Error::Network(net_err::Error::ProtocolError)),
            }

            // Limit body length to requested len; client may read more if server ignores range
            let chunk = &resp.body[..core::cmp::min(resp.body.len(), len)];
            if chunk.is_empty() {
                return Err(Error::Network(net_err::Error::ReadError));
            }

            // Write to storage at base_offset + start
            storage
                .write(base_offset + (start as u32), chunk)
                .map_err(|_| Error::Storage(storage_err::Error::WriteError))?;

            // Update CRC and counters
            crc.update(chunk);
            downloaded += chunk.len();

            // Progress
            if let Some(mp) = mqtt.as_deref_mut() {
                let _ = mp.publish_progress(Progress {
                    bytes_total: source.size,
                    bytes_downloaded: downloaded,
                    state: State::Downloading,
                });
            }
        }

        // Verify
        self.state = State::Verifying;
        if self.cfg.verify_crc32 {
            if let Some(expected) = source.crc32 {
                let actual = crc.finalize();
                if actual != expected {
                    self.state = State::Failed;
                    if let Some(mp) = mqtt.as_deref_mut() {
                        let _ = mp.publish_progress(Progress {
                            bytes_total: source.size,
                            bytes_downloaded: source.size,
                            state: State::Failed,
                        });
                    }
                    return Err(Error::VerifyFailed);
                }
            }
        }

        // Finalize
        self.state = State::Finalizing;
        if let Some(mp) = mqtt.as_deref_mut() {
            let _ = mp.publish_progress(Progress {
                bytes_total: source.size,
                bytes_downloaded: source.size,
                state: State::Finalizing,
            });
        }

        // Completed
        self.state = State::Completed;
        if let Some(mp) = mqtt.as_deref_mut() {
            let _ = mp.publish_progress(Progress {
                bytes_total: source.size,
                bytes_downloaded: source.size,
                state: State::Completed,
            });
        }
        Ok(())
    }
}

/// Helper for reporting OTA progress via MQTT.
pub struct MqttProgress<'a, C: crate::network::Connection> {
    client: &'a mut MqttClient<C>,
    /// Topic to publish progress messages
    topic: &'a str,
}

impl<'a, C: crate::network::Connection> MqttProgress<'a, C> {
    pub fn new(client: &'a mut MqttClient<C>, topic: &'a str) -> Self {
        Self { client, topic }
    }

    fn publish_progress(&mut self, p: Progress) -> Result<(), Error> {
        // Build tiny JSON using serde-json-core
        #[derive(serde::Serialize)]
        struct Body<'b> {
            bytes: usize,
            total: usize,
            state: &'b str,
        }

        let state_str = match p.state {
            State::Idle => "idle",
            State::Erasing => "erasing",
            State::Downloading => "downloading",
            State::Verifying => "verifying",
            State::Finalizing => "finalizing",
            State::Completed => "completed",
            State::Failed => "failed",
            State::Canceled => "canceled",
        };

        let body = Body {
            bytes: p.bytes_downloaded,
            total: p.bytes_total,
            state: state_str,
        };
        let encoded: Vec<u8, 128> = serde_json_core::to_vec(&body).map_err(|_| Error::Protocol)?;
        self.client
            .publish(self.topic, &encoded, QoS::AtMostOnce)
            .map_err(Error::from)
    }
}
