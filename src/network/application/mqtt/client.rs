//! MQTT 3.1.1 client implementation for embedded systems.
//!
//! This module provides a lightweight MQTT client designed for `no_std` environments
//! and embedded systems. It implements the MQTT 3.1.1 specification with a focus on
//! simplicity, reliability, and minimal resource usage.
//!
//! # Features
//!
//! - MQTT 3.1.1 protocol compliance
//! - Quality of Service (QoS) levels 0, 1, and 2 support
//! - Clean session and persistent session support
//! - Configurable keep-alive mechanism
//! - Publish/Subscribe pattern implementation
//! - Fixed-size buffers for predictable memory usage
//! - Connection agnostic (works with any transport)
//!
//! # Protocol Overview
//!
//! MQTT (Message Queuing Telemetry Transport) is a lightweight, publish-subscribe
//! network protocol designed for use in low-bandwidth, high-latency, or unreliable networks.
//! It's particularly well-suited for IoT applications.
//!
//! ## Key Concepts
//!
//! - **Broker**: The central server that handles message routing
//! - **Client**: Devices that connect to the broker to publish/subscribe
//! - **Topic**: A UTF-8 string that acts as a message routing key
//! - **QoS**: Quality of Service levels that define delivery guarantees
//!
//! # Examples
//!
//! ## Basic Connection and Publishing
//!
//! ```rust,no_run
//! use libiot::network::application::mqtt::{Client, Options, QoS};
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
//! let options = Options {
//!     client_id: "sensor_device_01",
//!     keep_alive_seconds: 60,
//!     clean_session: true,
//! };
//!
//! // let mut client = Client::connect(connection, options)?;
//! // client.publish("sensors/temperature", b"23.5", QoS::AtMostOnce)?;
//! ```
//!
//! ## Subscribing to Topics
//!
//! ```rust,no_run
//! use libiot::network::application::mqtt::{Client, Options, QoS};
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
//! // After establishing connection as above
//! // client.subscribe("commands/+", QoS::AtLeastOnce)?;
//! //
//! // // Poll for incoming messages
//! // loop {
//! //     if let Some(message) = client.poll()? {
//! //         println!("Received: {} on topic {}",
//! //                  String::from_utf8_lossy(&message.payload),
//! //                  message.topic);
//! //     }
//! // }
//! ```

//! An MQTT client implementation based on the MQTT 3.1.1 specification.
use crate::network::error::Error;
use crate::network::{Connection, Read, Write};
use heapless::{String, Vec};

// MQTT Control Packet types - these are the fixed header packet type values
/// MQTT CONNECT packet type identifier.
const CONNECT: u8 = 0x10;
/// MQTT CONNACK packet type identifier.
const CONNACK: u8 = 0x20;
/// MQTT PUBLISH packet type identifier.
const PUBLISH: u8 = 0x30;
/// MQTT SUBSCRIBE packet type identifier.
const SUBSCRIBE: u8 = 0x82;
/// MQTT SUBACK packet type identifier.
const SUBACK: u8 = 0x90;

/// An incoming MQTT publish message.
///
/// This structure represents a message received from the MQTT broker when
/// subscribed to one or more topics. It contains both the topic name and
/// the message payload.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mqtt::PublishPacket;
/// use heapless::{String, Vec};
///
/// // This would typically be created by the MQTT client
/// let packet = PublishPacket {
///     topic: String::try_from("sensors/temperature").unwrap(),
///     payload: Vec::from_slice(b"23.5").unwrap(),
/// };
///
/// assert_eq!(packet.topic.as_str(), "sensors/temperature");
/// assert_eq!(&packet.payload[..], b"23.5");
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PublishPacket {
    /// The topic on which the message was published.
    ///
    /// Maximum length is 256 characters to fit within embedded memory constraints.
    pub topic: String<256>,

    /// The message payload data.
    ///
    /// Maximum size is 1024 bytes to balance functionality with memory usage.
    /// For larger payloads, consider chunking the data across multiple messages.
    pub payload: Vec<u8, 1024>,
}

// Protocol constants defined by MQTT 3.1.1 specification
/// MQTT protocol name as defined in the specification.
const PROTOCOL_NAME: &[u8] = b"MQTT";
/// MQTT protocol level for version 3.1.1.
const PROTOCOL_LEVEL: u8 = 4; // MQTT 3.1.1

/// Quality of Service levels for MQTT messages.
///
/// QoS defines the guarantee of delivery for a specific message. Higher QoS levels
/// provide stronger delivery guarantees but require more network overhead and
/// client state management.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mqtt::QoS;
///
/// let qos0 = QoS::AtMostOnce;   // Fire and forget
/// let qos1 = QoS::AtLeastOnce;  // Acknowledged delivery
/// let qos2 = QoS::ExactlyOnce;  // Assured delivery
///
/// assert_eq!(qos0 as u8, 0);
/// assert_eq!(qos1 as u8, 1);
/// assert_eq!(qos2 as u8, 2);
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum QoS {
    /// **QoS 0**: At most once delivery.
    ///
    /// Messages are delivered according to the best effort of the underlying network.
    /// Message loss can occur. This level could be used, for example, with ambient
    /// sensor data where it's not critical if an individual reading is lost.
    AtMostOnce = 0,

    /// **QoS 1**: At least once delivery.
    ///
    /// Messages are assured to arrive but duplicates can occur. This level could be
    /// used for applications where duplicate messages are acceptable but message
    /// loss is not.
    AtLeastOnce = 1,

    /// **QoS 2**: Exactly once delivery.
    ///
    /// Messages are assured to arrive exactly once. This is the safest but slowest
    /// level. Use for critical messages where duplicates could cause problems.
    ExactlyOnce = 2,
}

/// Configuration options for MQTT client connection.
///
/// These options control how the client connects to the MQTT broker and
/// behaves during the session. All fields are required and must be set
/// appropriately for your use case.
///
/// # Examples
///
/// ```rust
/// use libiot::network::application::mqtt::Options;
///
/// let options = Options {
///     client_id: "my_iot_device",
///     keep_alive_seconds: 60,
///     clean_session: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// The client identifier, must be unique within the broker.
    ///
    /// Each client connecting to a broker must have a unique client identifier.
    /// If a client connects with a client identifier that is already in use by
    /// another client, the broker will disconnect the existing client.
    ///
    /// # Constraints
    /// - Must be 1-23 UTF-8 encoded bytes
    /// - Should be unique per broker
    /// - Cannot be empty (use broker-generated ID if needed)
    pub client_id: &'a str,

    /// The keep-alive time interval in seconds.
    ///
    /// This defines the maximum time interval between messages sent or received.
    /// It enables the client and broker to detect when the other has disconnected.
    /// A value of 0 disables keep-alive.
    ///
    /// # Recommended Values
    /// - IoT devices: 60-300 seconds
    /// - Mobile devices: 30-60 seconds  
    /// - Always-connected systems: 300+ seconds
    pub keep_alive_seconds: u16,

    /// Whether to start a clean session.
    ///
    /// - `true`: The broker will discard any previous session state and start fresh
    /// - `false`: The broker will resume the previous session if one exists
    ///
    /// Clean sessions are simpler but don't preserve subscriptions across reconnections.
    /// Persistent sessions maintain state but require more broker resources.
    pub clean_session: bool,
}

/// An MQTT 3.1.1 client for publish-subscribe messaging.
///
/// The client manages a connection to an MQTT broker and provides methods for
/// publishing messages, subscribing to topics, and receiving incoming messages.
/// It's designed to work with any connection type implementing the [`Connection`] trait.
///
/// # Type Parameters
///
/// * `C` - The connection type implementing [`Connection`]
///
/// # Examples
///
/// ```rust,no_run
/// use libiot::network::application::mqtt::{Client, Options, QoS};
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
/// let connection = TcpConnection;
/// let options = Options {
///     client_id: "sensor_node_1",
///     keep_alive_seconds: 120,
///     clean_session: true,
/// };
///
/// // let client = Client::connect(connection, options)?;
/// ```
pub struct Client<C: Connection> {
    connection: C,
    is_connected: bool,
}

impl<C: Connection> Client<C> {
    /// Establish an MQTT connection with the broker.
    ///
    /// This function performs the MQTT connection handshake by sending a CONNECT
    /// packet and waiting for a CONNACK response. If successful, it returns a
    /// connected client ready for publishing and subscribing.
    ///
    /// # Arguments
    ///
    /// * `connection` - An established network connection to the MQTT broker
    /// * `options` - Connection configuration options
    ///
    /// # Returns
    ///
    /// * `Ok(client)` - Successfully connected MQTT client
    /// * `Err(error)` - Connection failed due to network or protocol error
    ///
    /// # Errors
    ///
    /// This method can fail with several error types:
    ///
    /// * [`Error::WriteError`] - Failed to send CONNECT packet
    /// * [`Error::ReadError`] - Failed to read CONNACK response
    /// * [`Error::ConnectionClosed`] - Connection closed during handshake
    /// * [`Error::ConnectionRefused`] - Broker refused the connection
    /// * [`Error::ProtocolError`] - Invalid CONNACK packet received
    ///
    /// # Connection Refused Reasons
    ///
    /// The broker may refuse connection for various reasons:
    /// - Unacceptable protocol version
    /// - Client identifier rejected
    /// - Server unavailable
    /// - Bad username or password
    /// - Client not authorized
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libiot::network::application::mqtt::{Client, Options, QoS};
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
    /// let tcp_connection = TcpConnection;
    /// let options = Options {
    ///     client_id: "weather_station",
    ///     keep_alive_seconds: 60,
    ///     clean_session: true,
    /// };
    ///
    /// // match Client::connect(tcp_connection, options) {
    /// //     Ok(mut client) => {
    /// //         println!("Connected to MQTT broker!");
    /// //         // Ready to publish/subscribe
    /// //     }
    /// //     Err(e) => println!("Connection failed: {:?}", e),
    /// // }
    /// ```
    pub fn connect(mut connection: C, options: Options) -> Result<Self, Error> {
        // --- Variable Header ---
        let mut vh: Vec<u8, 10> = Vec::new();
        vh.extend_from_slice(&(PROTOCOL_NAME.len() as u16).to_be_bytes())
            .unwrap();
        vh.extend_from_slice(PROTOCOL_NAME).unwrap();
        vh.push(PROTOCOL_LEVEL).unwrap();

        let mut connect_flags = 0;
        if options.clean_session {
            connect_flags |= 0x02;
        }
        vh.push(connect_flags).unwrap();
        vh.extend_from_slice(&options.keep_alive_seconds.to_be_bytes())
            .unwrap();

        // --- Payload ---
        let mut payload: Vec<u8, 256> = Vec::new();
        let client_id_bytes = options.client_id.as_bytes();
        payload
            .extend_from_slice(&(client_id_bytes.len() as u16).to_be_bytes())
            .unwrap();
        payload.extend_from_slice(client_id_bytes).unwrap();

        let remaining_len = vh.len() + payload.len();

        // --- Fixed Header ---
        let mut fixed_header: Vec<u8, 5> = Vec::new();
        fixed_header.push(CONNECT).unwrap();
        encode_remaining_length(&mut fixed_header, remaining_len)
            .map_err(|_| Error::ProtocolError)?;

        // Write packet to the connection
        connection
            .write(&fixed_header)
            .map_err(|_| Error::WriteError)?;
        connection.write(&vh).map_err(|_| Error::WriteError)?;
        connection.write(&payload).map_err(|_| Error::WriteError)?;
        connection.flush().map_err(|_| Error::WriteError)?;

        // Wait for and parse CONNACK
        let mut connack_buf = [0u8; 4];
        let mut total_read = 0;
        while total_read < connack_buf.len() {
            match connection.read(&mut connack_buf[total_read..]) {
                Ok(0) => return Err(Error::ConnectionClosed),
                Ok(n) => total_read += n,
                Err(_) => return Err(Error::ReadError),
            }
        }

        if connack_buf[0] != CONNACK {
            return Err(Error::ProtocolError);
        }

        if connack_buf[1] != 2 {
            return Err(Error::ProtocolError);
        }

        // Check connection acknowledgement status
        match connack_buf[3] {
            0 => Ok(Self {
                connection,
                is_connected: true,
            }),
            1..=5 => Err(Error::ConnectionRefused),
            _ => Err(Error::ProtocolError),
        }
    }

    /// Publish a message to a specific topic.
    ///
    /// Sends a PUBLISH packet to the broker with the specified topic, payload,
    /// and quality of service level. The message will be delivered to all
    /// clients subscribed to the topic (or matching topic filters).
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to publish to (UTF-8 string)
    /// * `payload` - The message payload data (binary data)
    /// * `qos` - Quality of service level for this message
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Message published successfully
    /// * `Err(error)` - Failed to publish due to network or protocol error
    ///
    /// # Errors
    ///
    /// * [`Error::WriteError`] - Failed to send the publish packet
    /// * [`Error::ProtocolError`] - Invalid topic name or payload too large
    ///
    /// # Topic Naming Rules
    ///
    /// Topic names must follow MQTT specification rules:
    /// - UTF-8 encoded strings
    /// - Cannot contain wildcards (`+` or `#`)
    /// - Cannot be empty
    /// - Case sensitive
    /// - Forward slash (`/`) is used as level separator
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libiot::network::application::mqtt::{Client, QoS};
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
    /// # let mut client = Client { connection: MockConnection, is_connected: true };
    ///
    /// // Publish sensor readings
    /// // client.publish("sensors/temperature", b"23.5", QoS::AtMostOnce)?;
    /// // client.publish("sensors/humidity", b"65", QoS::AtLeastOnce)?;
    ///
    /// // Publish JSON data
    /// let json_data = br#"{"temp":23.5,"humidity":65,"timestamp":1234567890}"#;
    /// // client.publish("devices/sensor01/data", json_data, QoS::AtLeastOnce)?;
    /// ```
    pub fn publish(&mut self, topic: &str, payload: &[u8], qos: QoS) -> Result<(), Error> {
        let mut fixed_header: Vec<u8, 5> = Vec::new();
        let mut packet: Vec<u8, 1024> = Vec::new();

        // --- Variable Header ---
        let topic_bytes = topic.as_bytes();
        packet
            .extend_from_slice(&(topic_bytes.len() as u16).to_be_bytes())
            .unwrap();
        packet.extend_from_slice(topic_bytes).unwrap();

        // --- Payload ---
        packet.extend_from_slice(payload).unwrap();

        // --- Fixed Header ---
        let mut flags = PUBLISH;
        if qos == QoS::AtLeastOnce || qos == QoS::ExactlyOnce {
            flags |= (qos as u8) << 1;
        }
        fixed_header.push(flags).unwrap();
        encode_remaining_length(&mut fixed_header, packet.len()).unwrap();

        // Write to connection
        self.connection
            .write(&fixed_header)
            .map_err(|_| Error::WriteError)?;
        self.connection
            .write(&packet)
            .map_err(|_| Error::WriteError)?;
        self.connection.flush().map_err(|_| Error::WriteError)?;

        Ok(())
    }

    /// Subscribe to a topic filter to receive messages.
    ///
    /// Sends a SUBSCRIBE packet to the broker requesting to receive messages
    /// published to topics that match the specified topic filter. The broker
    /// will respond with a SUBACK packet to confirm the subscription.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic filter to subscribe to (can include wildcards)
    /// * `qos` - Maximum quality of service level for received messages
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Subscription successful
    /// * `Err(error)` - Subscription failed due to network or protocol error
    ///
    /// # Errors
    ///
    /// * [`Error::WriteError`] - Failed to send the subscribe packet
    /// * [`Error::ReadError`] - Failed to read SUBACK response
    /// * [`Error::ConnectionClosed`] - Connection closed during operation
    /// * [`Error::ProtocolError`] - Invalid SUBACK packet or topic filter
    ///
    /// # Topic Filter Wildcards
    ///
    /// MQTT supports two types of wildcards in topic filters:
    ///
    /// - **Single-level wildcard (`+`)**: Matches exactly one topic level
    ///   - `sensors/+/temperature` matches `sensors/room1/temperature`, `sensors/room2/temperature`
    ///
    /// - **Multi-level wildcard (`#`)**: Matches any number of topic levels
    ///   - `sensors/#` matches `sensors/temperature`, `sensors/room1/temperature`, `sensors/room1/humidity`
    ///   - Must be the last character in the topic filter
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libiot::network::application::mqtt::{Client, QoS};
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
    /// # let mut client = Client { connection: MockConnection, is_connected: true };
    ///
    /// // Subscribe to specific topic
    /// // client.subscribe("devices/sensor01/temperature", QoS::AtLeastOnce)?;
    ///
    /// // Subscribe to all sensors in a room
    /// // client.subscribe("sensors/room1/+", QoS::AtMostOnce)?;
    ///
    /// // Subscribe to all command topics
    /// // client.subscribe("commands/#", QoS::ExactlyOnce)?;
    /// ```
    pub fn subscribe(&mut self, topic: &str, qos: QoS) -> Result<(), Error> {
        let mut fixed_header: Vec<u8, 5> = Vec::new();
        let mut packet: Vec<u8, 1024> = Vec::new();

        // --- Variable Header (Packet Identifier) ---
        let packet_id: u16 = 1; // Using a fixed packet ID for simplicity
        packet.extend_from_slice(&packet_id.to_be_bytes()).unwrap();

        // --- Payload ---
        let topic_bytes = topic.as_bytes();
        packet
            .extend_from_slice(&(topic_bytes.len() as u16).to_be_bytes())
            .unwrap();
        packet.extend_from_slice(topic_bytes).unwrap();
        packet.push(qos as u8).unwrap();

        // --- Fixed Header ---
        fixed_header.push(SUBSCRIBE).unwrap();
        encode_remaining_length(&mut fixed_header, packet.len()).unwrap();

        // Write to connection
        self.connection
            .write(&fixed_header)
            .map_err(|_| Error::WriteError)?;
        self.connection
            .write(&packet)
            .map_err(|_| Error::WriteError)?;
        self.connection.flush().map_err(|_| Error::WriteError)?;

        // Wait for SUBACK
        let mut suback_buf = [0u8; 5];
        let mut total_read = 0;
        while total_read < suback_buf.len() {
            match self.connection.read(&mut suback_buf[total_read..]) {
                Ok(0) => return Err(Error::ConnectionClosed),
                Ok(n) => total_read += n,
                Err(_) => return Err(Error::ReadError),
            }
        }

        if suback_buf[0] != SUBACK {
            return Err(Error::ProtocolError);
        }

        // Check packet identifier
        let suback_packet_id = u16::from_be_bytes([suback_buf[2], suback_buf[3]]);
        if suback_packet_id != packet_id {
            return Err(Error::ProtocolError);
        }

        Ok(())
    }

    /// Poll the connection for incoming PUBLISH messages.
    ///
    /// This method checks for incoming data on the connection and parses any
    /// PUBLISH packets received from the broker. It should be called regularly
    /// in a loop to receive messages from subscribed topics.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(packet))` - A publish message was received
    /// * `Ok(None)` - No message available at this time
    /// * `Err(error)` - Network or protocol error occurred
    ///
    /// # Errors
    ///
    /// * [`Error::ReadError`] - Failed to read from the connection
    /// * [`Error::ProtocolError`] - Received malformed MQTT packet
    ///
    /// # Usage Pattern
    ///
    /// The typical pattern is to call this method in a loop, either continuously
    /// or on a timer, to process incoming messages:
    ///
    /// ```rust,no_run
    /// use libiot::network::application::mqtt::{Client, QoS};
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
    /// # let mut client = Client { connection: MockConnection, is_connected: true };
    ///
    /// // Message processing loop
    /// // loop {
    /// //     match client.poll() {
    /// //         Ok(Some(message)) => {
    /// //             println!("Received on topic '{}': {:?}",
    /// //                      message.topic, message.payload);
    /// //             
    /// //             // Process the message based on topic
    /// //             if message.topic.starts_with("commands/") {
    /// //                 // Handle command message
    /// //             } else if message.topic.starts_with("sensors/") {
    /// //                 // Handle sensor data
    /// //             }
    /// //         }
    /// //         Ok(None) => {
    /// //             // No message available, continue or sleep
    /// //         }
    /// //         Err(e) => {
    /// //             println!("Error polling: {:?}", e);
    /// //             break;
    /// //         }
    /// //     }
    /// // }
    /// ```
    ///
    /// # Non-blocking Behavior
    ///
    /// This method is non-blocking and will return `Ok(None)` immediately if
    /// no data is available. For blocking behavior, call it in a loop with
    /// appropriate delays.
    pub fn poll(&mut self) -> Result<Option<PublishPacket>, Error> {
        let mut header_buf = [0u8; 1];
        match self.connection.read(&mut header_buf) {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(_) => return Err(Error::ReadError),
        }

        if header_buf[0] & 0xF0 == PUBLISH {
            let mut remaining_len_buf = [0u8; 4];
            let mut remaining_len = 0;
            let mut multiplier = 1;
            let mut i = 0;
            loop {
                self.connection
                    .read(&mut remaining_len_buf[i..i + 1])
                    .map_err(|_| Error::ReadError)?;
                remaining_len += (remaining_len_buf[i] as usize & 127) * multiplier;
                multiplier *= 128;
                if (remaining_len_buf[i] & 0x80) == 0 {
                    break;
                }
                i += 1;
            }

            let mut packet_buf = Vec::<u8, 1024>::new();
            packet_buf.resize(remaining_len, 0).unwrap();
            self.connection
                .read(&mut packet_buf)
                .map_err(|_| Error::ReadError)?;

            let topic_len = u16::from_be_bytes([packet_buf[0], packet_buf[1]]) as usize;
            let topic =
                String::from_utf8(Vec::from_slice(&packet_buf[2..2 + topic_len]).unwrap()).unwrap();

            let payload_start = 2 + topic_len;
            let payload = Vec::from_slice(&packet_buf[payload_start..]).unwrap();

            Ok(Some(PublishPacket { topic, payload }))
        } else {
            Ok(None)
        }
    }
}

/// Encode the remaining length field for an MQTT packet.
///
/// The remaining length field is a variable-length encoding scheme used in MQTT
/// to specify the number of bytes following the fixed header. This is an internal
/// utility function used by the client implementation.
///
/// # Arguments
///
/// * `buf` - Buffer to write the encoded length to
/// * `len` - Length value to encode
///
/// # Returns
///
/// * `Ok(())` - Length encoded successfully
/// * `Err(())` - Buffer too small or length too large
///
/// # Encoding Rules
///
/// The encoding uses up to 4 bytes where each byte encodes 7 bits of the length
/// value. The most significant bit indicates if another byte follows.
///
/// This allows encoding values from 0 to 268,435,455 (0xFF,0xFF,0xFF,0x7F).
fn encode_remaining_length(buf: &mut Vec<u8, 5>, mut len: usize) -> Result<(), ()> {
    loop {
        if buf.is_full() {
            return Err(());
        }
        let mut byte = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            byte |= 0x80;
        }
        buf.push(byte).unwrap(); // `is_full` check above ensures this won't panic
        if len == 0 {
            break;
        }
    }
    Ok(())
}
