use dotenvy::dotenv;
use libiot::network::protocol::mqtt::client::{Client, Options};
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
        Ok(())
    }
}

impl Connection for NetConnection {}

#[test]
fn test_connect_to_public_broker() {
    dotenv().ok();
    let address = env::var("TEST_MQTT_ADDRESS").unwrap_or("test.mosquitto.org:1883".to_string());
    let stream = TcpStream::connect(address).expect("Failed to connect to broker");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let conn = NetConnection { stream };

    let opts = Options {
        client_id: "libiot-test-client-12345",
        keep_alive_seconds: 10,
        clean_session: true,
    };

    let client = Client::connect(conn, opts);
    assert!(client.is_ok());
}

#[test]
fn test_publish_and_subscribe() {
    dotenv().ok();
    let address = env::var("TEST_MQTT_ADDRESS").unwrap_or("test.mosquitto.org:1883".to_string());
    let stream = TcpStream::connect(address).expect("Failed to connect to broker");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let conn = NetConnection { stream };

    let opts = Options {
        client_id: "libiot-test-client-67890",
        keep_alive_seconds: 10,
        clean_session: true,
    };

    let mut client = Client::connect(conn, opts).expect("Failed to connect");

    let topic = "libiot/test-topic";
    let payload = b"hello world";
    let qos = libiot::network::protocol::mqtt::client::QoS::AtMostOnce;

    client.subscribe(topic, qos).expect("Failed to subscribe");

    client
        .publish(topic, payload, qos)
        .expect("Failed to publish");

    // Poll for the message
    let packet = client.poll().expect("Failed to poll");

    assert!(packet.is_some());
    let publish_packet = packet.unwrap();
    assert_eq!(publish_packet.topic.as_str(), topic);
    assert_eq!(publish_packet.payload, payload);
}
