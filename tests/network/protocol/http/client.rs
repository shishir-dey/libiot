use dotenvy::dotenv;
use libiot::network::protocol::http::client::{Client, Method, Request};
use libiot::network::{Close, Connection, Read, Write};
use std::env;
use std::io::{Read as StdRead, Write as StdWrite};
use std::net::TcpStream;

struct NetConnection {
    stream: TcpStream,
}

impl Read for NetConnection {
    type Error = libiot::network::error::Error;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.stream.read(buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                libiot::network::error::Error::Timeout
            } else {
                libiot::network::error::Error::ReadError
            }
        })
    }
}

impl Write for NetConnection {
    type Error = libiot::network::error::Error;
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.stream
            .write(buf)
            .map_err(|_| libiot::network::error::Error::WriteError)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.stream
            .flush()
            .map_err(|_| libiot::network::error::Error::WriteError)
    }
}

impl Close for NetConnection {
    type Error = libiot::network::error::Error;
    fn close(self) -> Result<(), Self::Error> {
        self.stream.shutdown(std::net::Shutdown::Both).unwrap();
        Ok(())
    }
}

impl Connection for NetConnection {}

#[test]
fn test_http_get() {
    dotenv().ok();
    let address = env::var("TEST_HTTP_ADDRESS").unwrap_or("httpbin.org:80".to_string());
    let stream = TcpStream::connect(address.as_str()).expect("Failed to connect to server");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let conn = NetConnection { stream };
    let mut client = Client::new(conn);

    let mut headers = heapless::Vec::new();
    headers
        .push(libiot::network::protocol::http::client::Header {
            name: heapless::String::try_from("Host").unwrap(),
            value: heapless::String::try_from(address.as_str()).unwrap(),
        })
        .unwrap();

    let request = Request {
        method: Method::Get,
        path: "/get",
        headers,
        body: None,
    };

    let response = client.request(&request);
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status_code, 200);
}

#[test]
fn test_http_post() {
    dotenv().ok();
    let address = env::var("TEST_HTTP_ADDRESS").unwrap_or("httpbin.org:80".to_string());
    let stream = TcpStream::connect(address.as_str()).expect("Failed to connect to server");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let conn = NetConnection { stream };
    let mut client = Client::new(conn);

    let mut headers = heapless::Vec::new();
    headers
        .push(libiot::network::protocol::http::client::Header {
            name: heapless::String::try_from("Host").unwrap(),
            value: heapless::String::try_from(address.as_str()).unwrap(),
        })
        .unwrap();
    headers
        .push(libiot::network::protocol::http::client::Header {
            name: heapless::String::try_from("Content-Type").unwrap(),
            value: heapless::String::try_from("application/json").unwrap(),
        })
        .unwrap();

    let body = r#"{"hello":"world"}"#;

    let request = Request {
        method: Method::Post,
        path: "/post",
        headers,
        body: Some(body.as_bytes()),
    };

    let response = client.request(&request);
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status_code, 200);
}
