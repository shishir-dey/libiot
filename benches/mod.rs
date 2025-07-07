use criterion::{criterion_group, criterion_main};

mod network;

criterion_group!(
    benches,
    network::protocol::mqtt::client::bench_publish,
    network::protocol::mqtt::client::bench_poll,
    network::protocol::mqtt::client::bench_publish_and_poll_qos0,
    network::protocol::mqtt::client::bench_publish_and_poll_qos1
);
criterion_main!(benches);
