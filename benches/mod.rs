use criterion::{criterion_group, criterion_main};

mod network;

criterion_group!(
    benches,
    network::application::mqtt::client::bench_publish,
    network::application::mqtt::client::bench_poll,
    network::application::mqtt::client::bench_publish_and_poll_qos0,
    network::application::mqtt::client::bench_publish_and_poll_qos1
);
criterion_main!(benches);
