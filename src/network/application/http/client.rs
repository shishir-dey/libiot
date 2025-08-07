//! HTTP/1.1 client implementation for embedded systems.
//!
//! This module provides a lightweight HTTP client designed for `no_std` environments.
//! It supports basic HTTP operations with fixed-size buffers and minimal memory usage.
//!
//! # Features
//!
//! - HTTP/1.1 protocol support
//! - GET and POST methods
//! - Custom headers
//! - Request/response body handling
//! - Connection reuse
//! - Fixed-size buffers for predictable memory usage
//!
//! # Limitations
//!
//! - Only supports HTTP/1.1 (no HTTP/2 or HTTP/3)
//! - Limited to GET and POST methods
//! - Maximum header count and sizes are compile-time constants
//! - Response body size is limited by buffer capacity
//! - No automatic redirect handling
//! - No persistent connection management
//!
//! # Examples
//!
//! ## Simple GET Request
//!
//! ```rust,no_run
//! use libiot::network::application::http::client::{Client, Request, Method};
//! # use libiot::network::Connection;
//! # struct MockConnection;
//! # impl Connection for MockConnection {}
//! # impl libiot::network::Read for MockConnection {
//! #     type Error = ();
//! #     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> { Ok(0) }
//! # }
//! # impl libiot::network::Write for MockConnection {
//! #     type Error = ();
//! #     fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> { Ok(0) }
//! #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
//! # }
//! # impl libiot::network::Close for MockConnection {
//! #     type Error = ();
//! #     fn close(self) -> Result<(), Self::Error> { Ok(()) }
//! # }
//!
//! let connection = MockConnection;
//! let mut client = Client::new(connection);
//!
//! let request = Request {
//!     method: Method::Get,
//!     path: "/api/sensors",
//!     headers: heapless::Vec::new(),
//!     body: None,
//! };
//!
//! // let response = client.request(&request)?;
//! // println!("Status: {}", response.status_code);
//! ```
//!
//! ## POST Request with JSON Body
//!
//! ```rust,no_run
//! use libiot::network::application::http::client::{Client, Request, Method, Header};
//! # use libiot::network::Connection;
//! # struct MockConnection;
//! # impl Connection for MockConnection {}
//! # impl libiot::network::Read for MockConnection {
//! #     type Error = ();
//! #     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> { Ok(0) }
//! # }
//! # impl libiot::network::Write for MockConnection {
//! #     type Error = ();
//! #     fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> { Ok(0) }
//! #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
//! # }
//! # impl libiot::network::Close for MockConnection {
//! #     type Error = ();
//! #     fn close(self) -> Result<(), Self::Error> { Ok(()) }
//! # }
//!
//! let connection = MockConnection;
//! let mut client = Client::new(connection);
//!
//! let json_data = br#"{"temperature": 23.5, "humidity": 65}"#;
//! let mut headers = heapless::Vec::new();
//!
//! let content_type_header = Header {
//!     name: heapless::String::try_from("Content-Type").unwrap(),
//!     value: heapless::String::try_from("application/json").unwrap(),
//! };
//! headers.push(content_type_header).unwrap();
//!
//! let request = Request {
//!     method: Method::Post,
//!     path: "/api/data",
//!     headers,
//!     body: Some(json_data),
//! };
//!
//! // let response = client.request(&request)?;
//! ```

use crate::network::Connection;
use crate::network::error::Error;
use core::fmt::Write;
use heapless::{String, Vec};

/// Maximum number of headers allowed per request/response.
const MAX_HEADERS: usize = 16;

/// Maximum length for header names in bytes.
const MAX_HEADER_NAME_LEN: usize = 64;

/// Maximum length for header values in bytes.
const MAX_HEADER_VALUE_LEN: usize = 256;

/// HTTP request methods supported by the client.
///
/// Currently supports the most common HTTP methods used in IoT applications.
/// Additional methods can be added as needed.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::http::Method;
///
/// let get_method = Method::Get;
/// let post_method = Method::Post;
///
/// assert_eq!(get_method.as_str(), "GET");
/// assert_eq!(post_method.as_str(), "POST");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// HTTP GET method for retrieving data.
    Get,
    /// HTTP POST method for sending data.
    Post,
}

impl Method {
    /// Convert the method to its string representation.
    ///
    /// Returns the standard HTTP method name as used in request lines.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libiot::network::application::http::Method;
    ///
    /// assert_eq!(Method::Get.as_str(), "GET");
    /// assert_eq!(Method::Post.as_str(), "POST");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }
}

/// An HTTP header consisting of a name-value pair.
///
/// Headers are used to pass additional information with HTTP requests and responses.
/// Both the name and value are stored as heap-free strings with compile-time size limits.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::http::Header;
/// use heapless::String;
///
/// let header = Header {
///     name: String::try_from("Content-Type").unwrap(),
///     value: String::try_from("application/json").unwrap(),
/// };
///
/// assert_eq!(header.name.as_str(), "Content-Type");
/// assert_eq!(header.value.as_str(), "application/json");
/// ```
#[derive(Debug, Clone)]
pub struct Header {
    /// The header name (e.g., "Content-Type", "Authorization").
    pub name: String<MAX_HEADER_NAME_LEN>,
    /// The header value (e.g., "application/json", "Bearer token123").
    pub value: String<MAX_HEADER_VALUE_LEN>,
}

/// An HTTP request to be sent by the client.
///
/// Contains all the information needed to construct a complete HTTP request,
/// including method, path, headers, and optional body data.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::http::{Request, Method};
/// use heapless::Vec;
///
/// let request = Request {
///     method: Method::Get,
///     path: "/api/status",
///     headers: Vec::new(),
///     body: None,
/// };
/// ```
pub struct Request<'a> {
    /// The HTTP method to use for this request.
    pub method: Method,
    /// The request path (e.g., "/api/data", "/status").
    pub path: &'a str,
    /// Optional headers to include with the request.
    pub headers: Vec<Header, MAX_HEADERS>,
    /// Optional request body data.
    pub body: Option<&'a [u8]>,
}

/// An HTTP response received from the server.
///
/// Contains the response status code, headers, and body data returned by the server.
/// The body size is limited by the internal buffer capacity.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::http::Response;
///
/// // Response is typically created by the HTTP client
/// // let response = client.request(&request)?;
/// //
/// // if response.status_code == 200 {
/// //     println!("Success! Body length: {}", response.body.len());
/// // }
/// ```
#[derive(Debug)]
pub struct Response {
    /// HTTP status code (e.g., 200, 404, 500).
    pub status_code: u16,
    /// Response headers sent by the server.
    pub headers: Vec<Header, MAX_HEADERS>,
    /// Response body data with a maximum size of 2048 bytes.
    pub body: Vec<u8, 2048>,
}

/// HTTP client for making requests over any connection type.
///
/// The client is generic over the connection type, allowing it to work with
/// different transport layers (TCP, TLS, etc.) as long as they implement
/// the [`Connection`] trait.
///
/// # Type Parameters
///
/// * `C` - The connection type implementing [`Connection`]
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::network::application::http::Client;
/// # use libiot::network::Connection;
/// # struct MockConnection;
/// # impl Connection for MockConnection {}
/// # impl libiot::network::Read for MockConnection {
/// #     type Error = ();
/// #     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> { Ok(0) }
/// # }
/// # impl libiot::network::Write for MockConnection {
/// #     type Error = ();
/// #     fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> { Ok(0) }
/// #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// # impl libiot::network::Close for MockConnection {
/// #     type Error = ();
/// #     fn close(self) -> Result<(), Self::Error> { Ok(()) }
/// # }
///
/// let connection = MockConnection;
/// let client = Client::new(connection);
/// // Now ready to make HTTP requests
/// ```
pub struct Client<C: Connection> {
    connection: C,
}

impl<C: Connection> Client<C> {
    /// Create a new HTTP client with the given connection.
    ///
    /// The connection should already be established to the target server.
    /// The client takes ownership of the connection and will use it for
    /// sending requests and receiving responses.
    ///
    /// # Arguments
    ///
    /// * `connection` - An established connection implementing [`Connection`]
    ///
    /// # Returns
    ///
    /// A new HTTP client ready to make requests.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libiot::network::application::http::Client;
    /// # use libiot::network::Connection;
    /// # struct TcpConnection;
    /// # impl Connection for TcpConnection {}
    /// # impl libiot::network::Read for TcpConnection {
    /// #     type Error = ();
    /// #     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> { Ok(0) }
    /// # }
    /// # impl libiot::network::Write for TcpConnection {
    /// #     type Error = ();
    /// #     fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> { Ok(0) }
    /// #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # impl libiot::network::Close for TcpConnection {
    /// #     type Error = ();
    /// #     fn close(self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    ///
    /// // Assume we have a TCP connection established
    /// let tcp_connection = TcpConnection;
    /// let mut http_client = Client::new(tcp_connection);
    /// ```
    pub fn new(connection: C) -> Self {
        Self { connection }
    }

    /// Send an HTTP request and receive the response.
    ///
    /// This method constructs a complete HTTP request from the provided [`Request`],
    /// sends it over the connection, and parses the response. The operation is
    /// synchronous and will block until the response is received or an error occurs.
    ///
    /// # Arguments
    ///
    /// * `request` - The HTTP request to send
    ///
    /// # Returns
    ///
    /// * `Ok(response)` - The HTTP response from the server
    /// * `Err(error)` - Network or protocol error occurred
    ///
    /// # Errors
    ///
    /// This method can return various errors:
    ///
    /// * [`Error::WriteError`] - Failed to send the request
    /// * [`Error::ReadError`] - Failed to read the response
    /// * [`Error::ConnectionClosed`] - Connection was closed unexpectedly
    /// * [`Error::ProtocolError`] - Invalid HTTP response format
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libiot::network::application::http::{Client, Request, Method};
    /// # use libiot::network::Connection;
    /// # struct MockConnection;
    /// # impl Connection for MockConnection {}
    /// # impl libiot::network::Read for MockConnection {
    /// #     type Error = ();
    /// #     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> { Ok(0) }
    /// # }
    /// # impl libiot::network::Write for MockConnection {
    /// #     type Error = ();
    /// #     fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> { Ok(0) }
    /// #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # impl libiot::network::Close for MockConnection {
    /// #     type Error = ();
    /// #     fn close(self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    ///
    /// let connection = MockConnection;
    /// let mut client = Client::new(connection);
    ///
    /// let request = Request {
    ///     method: Method::Get,
    ///     path: "/api/temperature",
    ///     headers: heapless::Vec::new(),
    ///     body: None,
    /// };
    ///
    /// // match client.request(&request) {
    /// //     Ok(response) => {
    /// //         if response.status_code == 200 {
    /// //             println!("Temperature data: {:?}", response.body);
    /// //         }
    /// //     }
    /// //     Err(e) => println!("Request failed: {:?}", e),
    /// // }
    /// ```
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

/// Find the first occurrence of a slice in another slice and return its starting position.
///
/// This is a utility function used internally for parsing HTTP responses to locate
/// the boundary between headers and body.
///
/// # Arguments
///
/// * `haystack` - The slice to search in
/// * `needle` - The slice to search for
///
/// # Returns
///
/// * `Some(index)` - The starting position of the first occurrence
/// * `None` - The needle was not found in the haystack
fn find_slice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
