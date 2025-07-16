use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use dotenvy::dotenv;
use libiot::network::application::mqtt::client::{Client, Options, QoS};
use libiot::network::{Close, Connection, Read, Write};
use std::env;
use std::io::{Read as StdRead, Write as StdWrite};
use std::net::TcpStream;
use std::time::{Duration, Instant};

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

fn setup_client(client_id: &str) -> Client<NetConnection> {
    dotenv().ok();
    let address = env::var("TEST_MQTT_ADDRESS").unwrap_or("test.mosquitto.org:1883".to_string());
    let stream = TcpStream::connect(address).expect("Failed to connect to broker");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let conn = NetConnection { stream };

    let opts = Options {
        client_id,
        keep_alive_seconds: 10,
        clean_session: true,
    };

    Client::connect(conn, opts).expect("Failed to connect")
}

pub fn bench_publish(c: &mut Criterion) {
    let mut group = c.benchmark_group("publish");
    let payload = b"hello from publish";
    group.throughput(Throughput::Bytes(payload.len() as u64));
    group.bench_function("publish", |b| {
        b.iter_batched_ref(
            || setup_client("libiot-bench-publish"),
            |client| {
                client
                    .publish("libiot/bench-topic", payload, QoS::AtMostOnce)
                    .expect("Failed to publish");
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

pub fn bench_poll(c: &mut Criterion) {
    let mut group = c.benchmark_group("poll");
    let payload = b"hello from poll";
    group.throughput(Throughput::Bytes(payload.len() as u64));
    group.bench_function("poll", |b| {
        b.iter_batched_ref(
            || {
                let mut client = setup_client("libiot-bench-poll");
                client
                    .subscribe("libiot/bench-topic", QoS::AtMostOnce)
                    .expect("Failed to subscribe");
                client
            },
            |client| {
                client
                    .publish("libiot/bench-topic", payload, QoS::AtMostOnce)
                    .expect("Failed to publish");
                let _ = client.poll().expect("Failed to poll");
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

pub fn bench_publish_and_poll_qos0(c: &mut Criterion) {
    let mut group = c.benchmark_group("publish_and_poll_qos0");
    let payload = b"hello world from bench";
    group.throughput(Throughput::Bytes(payload.len() as u64 * 50));

    group.bench_function("publish_and_poll_qos0", |b| {
        b.iter_batched_ref(
            || {
                let mut client = setup_client("libiot-bench-pubpoll-qos0");
                client
                    .subscribe("libiot/bench-topic-qos0", QoS::AtMostOnce)
                    .expect("Failed to subscribe");
                client
            },
            |client| {
                for _ in 0..50 {
                    client
                        .publish("libiot/bench-topic-qos0", payload, QoS::AtMostOnce)
                        .expect("Failed to publish");
                    let _ = client.poll().expect("Failed to poll");
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

pub fn bench_publish_and_poll_qos1(c: &mut Criterion) {
    let mut group = c.benchmark_group("publish_and_poll_qos1");
    let payload = b"hello world from bench qos1";
    group.throughput(Throughput::Bytes(payload.len() as u64 * 50));

    group.bench_function("publish_and_poll_qos1", |b| {
        b.iter_batched_ref(
            || {
                let mut client = setup_client("libiot-bench-pubpoll-qos1");
                client
                    .subscribe("libiot/bench-topic-qos1", QoS::AtLeastOnce)
                    .expect("Failed to subscribe");

                // Warm-up
                for _ in 0..5 {
                    client
                        .publish("libiot/bench-topic-qos1", payload, QoS::AtLeastOnce)
                        .expect("Failed to publish");
                    let _ = client.poll();
                    let _ = client.poll();
                }
                client
            },
            |client| {
                for _ in 0..50 {
                    client
                        .publish("libiot/bench-topic-qos1", payload, QoS::AtLeastOnce)
                        .expect("Failed to publish");
                    let _ = client.poll().expect("Failed to poll"); // Poll for puback
                    let _ = client.poll().expect("Failed to poll"); // Poll for message
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}
