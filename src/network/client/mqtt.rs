//! An MQTT client implementation based on the MQTT 3.1.1 specification.
use crate::network::error::Error;
use crate::network::{Connection, Read, Write};
use heapless::{String, Vec};

// MQTT Control Packet types
const CONNECT: u8 = 0x10;
const CONNACK: u8 = 0x20;
const PUBLISH: u8 = 0x30;
const SUBSCRIBE: u8 = 0x82;
const SUBACK: u8 = 0x90;

/// An incoming publish packet.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PublishPacket {
    /// The topic of the message.
    pub topic: String<256>,
    /// The payload of the message.
    pub payload: Vec<u8, 1024>,
}

// Protocol constants
const PROTOCOL_NAME: &[u8] = b"MQTT";
const PROTOCOL_LEVEL: u8 = 4; // MQTT 3.1.1

/// Quality of Service levels for MQTT messages.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum QoS {
    /// At most once delivery.
    AtMostOnce = 0,
    /// At least once delivery.
    AtLeastOnce = 1,
    /// Exactly once delivery.
    ExactlyOnce = 2,
}

/// Options for configuring the MQTT client connection.
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// The client identifier, must be unique.
    pub client_id: &'a str,
    /// The keep-alive time in seconds.
    pub keep_alive_seconds: u16,
    /// Whether to start a clean session.
    pub clean_session: bool,
}

/// An MQTT 3.1.1 client.
pub struct Client<C: Connection> {
    connection: C,
    is_connected: bool,
}

impl<C: Connection> Client<C> {
    /// Establishes an MQTT connection with the server.
    ///
    /// This function sends a `CONNECT` packet and waits for a `CONNACK` response.
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

    /// Publishes a message to a topic.
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

    /// Subscribes to a topic.
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

    /// Polls the connection for incoming messages.
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

/// Encodes the remaining length field for an MQTT packet.
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
