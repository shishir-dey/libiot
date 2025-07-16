use crate::network::Connection;
use crate::network::error::Error;
use core::fmt::Write;
use heapless::{String, Vec};

const MAX_HEADERS: usize = 16;
const MAX_HEADER_NAME_LEN: usize = 64;
const MAX_HEADER_VALUE_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

impl Method {
    fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Header {
    pub name: String<MAX_HEADER_NAME_LEN>,
    pub value: String<MAX_HEADER_VALUE_LEN>,
}

pub struct Request<'a> {
    pub method: Method,
    pub path: &'a str,
    pub headers: Vec<Header, MAX_HEADERS>,
    pub body: Option<&'a [u8]>,
}

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub headers: Vec<Header, MAX_HEADERS>,
    pub body: Vec<u8, 2048>,
}

pub struct Client<C: Connection> {
    connection: C,
}

impl<C: Connection> Client<C> {
    pub fn new(connection: C) -> Self {
        Self { connection }
    }

    pub fn request(&mut self, request: &Request) -> Result<Response, Error> {
        // --- Build Request ---
        let mut request_buf: Vec<u8, 2048> = Vec::new();

        // Request line
        request_buf
            .extend_from_slice(request.method.as_str().as_bytes())
            .map_err(|_| Error::WriteError)?;
        request_buf.push(b' ').map_err(|_| Error::WriteError)?;
        request_buf
            .extend_from_slice(request.path.as_bytes())
            .map_err(|_| Error::WriteError)?;
        request_buf
            .extend_from_slice(b" HTTP/1.1\r\n")
            .map_err(|_| Error::WriteError)?;

        // Headers
        let mut has_user_agent = false;
        for header in &request.headers {
            if header.name.eq_ignore_ascii_case("User-Agent") {
                has_user_agent = true;
            }
            request_buf
                .extend_from_slice(header.name.as_bytes())
                .map_err(|_| Error::WriteError)?;
            request_buf
                .extend_from_slice(b": ")
                .map_err(|_| Error::WriteError)?;
            request_buf
                .extend_from_slice(header.value.as_bytes())
                .map_err(|_| Error::WriteError)?;
            request_buf
                .extend_from_slice(b"\r\n")
                .map_err(|_| Error::WriteError)?;
        }

        if !has_user_agent {
            request_buf
                .extend_from_slice(b"User-Agent:;\r\n")
                .map_err(|_| Error::WriteError)?;
        }

        // Body
        if let Some(body) = request.body {
            let mut len_str: String<10> = String::new();
            write!(len_str, "{}", body.len()).unwrap();

            request_buf
                .extend_from_slice(b"Content-Length: ")
                .map_err(|_| Error::WriteError)?;
            request_buf
                .extend_from_slice(len_str.as_bytes())
                .map_err(|_| Error::WriteError)?;
            request_buf
                .extend_from_slice(b"\r\n\r\n")
                .map_err(|_| Error::WriteError)?;
            request_buf
                .extend_from_slice(body)
                .map_err(|_| Error::WriteError)?;
        } else {
            request_buf
                .extend_from_slice(b"\r\n")
                .map_err(|_| Error::WriteError)?;
        }

        // --- Send Request ---
        self.connection
            .write(&request_buf)
            .map_err(|_| Error::WriteError)?;
        self.connection.flush().map_err(|_| Error::WriteError)?;

        // --- Receive Response ---
        let mut response_buf = [0u8; 2048];
        let mut total_read = 0;
        loop {
            match self.connection.read(&mut response_buf[total_read..]) {
                Ok(0) if total_read > 0 => break, // Connection closed, but we have data
                Ok(0) => return Err(Error::ConnectionClosed),
                Ok(n) => {
                    total_read += n;
                    if total_read >= response_buf.len() {
                        break;
                    }
                    // This is a simplistic check. A robust client would parse Content-Length
                    // and continue reading until the body is fully received.
                    if find_slice(&response_buf[..total_read], b"\r\n\r\n").is_some() {
                        // For now, we assume the first read gets all headers and maybe start of body
                        break;
                    }
                }
                Err(_) => return Err(Error::ReadError),
            }
        }

        // --- Parse Response ---
        let response_data = &response_buf[..total_read];

        // Find where headers end and body begins
        let header_end_pos = find_slice(response_data, b"\r\n\r\n").ok_or(Error::ProtocolError)?;
        let header_data = &response_data[..header_end_pos];
        let body_data = &response_data[header_end_pos + 4..];

        let header_str = core::str::from_utf8(header_data).map_err(|_| Error::ProtocolError)?;
        let mut lines = header_str.lines();

        // Parse status line
        let status_line = lines.next().ok_or(Error::ProtocolError)?;
        let mut status_parts = status_line.splitn(3, ' ');
        status_parts.next(); // Skip HTTP version
        let status_code_str = status_parts.next().ok_or(Error::ProtocolError)?;
        let status_code = status_code_str
            .parse::<u16>()
            .map_err(|_| Error::ProtocolError)?;

        // Parse headers
        let mut response_headers: Vec<Header, MAX_HEADERS> = Vec::new();
        let mut content_length: Option<usize> = None;

        for line in lines {
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(2, ':');
            let name = parts.next().ok_or(Error::ProtocolError)?.trim();
            let value = parts.next().ok_or(Error::ProtocolError)?.trim();

            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = value.parse::<usize>().ok();
            }

            response_headers
                .push(Header {
                    name: String::try_from(name).map_err(|_| Error::ProtocolError)?,
                    value: String::try_from(value).map_err(|_| Error::ProtocolError)?,
                })
                .map_err(|_| Error::ProtocolError)?;
        }

        let mut body = Vec::from_slice(body_data).map_err(|_| Error::ProtocolError)?;
        if let Some(len) = content_length {
            while body.len() < len {
                if body.len() == body.capacity() {
                    // Body is larger than our buffer.
                    return Err(Error::ProtocolError);
                }

                // Read more data into a temporary buffer, then extend our body vec.
                let mut temp_buf = [0; 256];
                let remaining_len = len - body.len();
                let read_len = core::cmp::min(remaining_len, temp_buf.len());
                if read_len == 0 {
                    break;
                }

                match self.connection.read(&mut temp_buf[..read_len]) {
                    Ok(0) => return Err(Error::ConnectionClosed), // Prematurely closed
                    Ok(n) => {
                        if body.extend_from_slice(&temp_buf[..n]).is_err() {
                            return Err(Error::ProtocolError); // Should not happen given capacity check
                        }
                    }
                    Err(_) => return Err(Error::ReadError),
                }
            }

            // Truncate to ensure we have exactly `len` bytes.
            if body.len() > len {
                body.truncate(len);
            }
        }

        Ok(Response {
            status_code,
            headers: response_headers,
            body,
        })
    }
}

/// Finds the first occurrence of a slice in another slice and returns its starting position.
fn find_slice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
