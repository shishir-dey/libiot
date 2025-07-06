# libiot

A Rust SDK that enables any IoT device to securely connect to the cloud, manage data, and interact with cloud services.

## Project Structure

The project is organized as follows:

```
.
├── src
│   ├── lib.rs
│   ├── network
│   │   ├── error.rs
│   │   ├── mod.rs
│   │   ├── protocol
│   │   │   ├── mod.rs
│   │   │   ├── http
│   │   │   │   ├── client.rs
│   │   │   │   └── mod.rs
│   │   │   └── mqtt
│   │   │       ├── client.rs
│   │   │       └── mod.rs
│   ├── ota
│   ├── storage
│   │   ├── error.rs
│   │   └── mod.rs
│   └── system
├── tests
│   ├── network
│   │   └── protocol
│   │       └── http
│   │           ├── client.rs
│   │           └── mod.rs
│   │       └── mqtt
│   │           ├── client.rs
│   │           └── mod.rs
│   └── storage
│       └── mod.rs
├── .env
├── Cargo.toml
└── README.md
```

## Tech Stack

- **Rust**: The core programming language.
- **`#![no_std]`**: Designed for embedded systems without a standard library.
- **`futures`**: Used for testing asynchronous code.

## Acknowledgments

This library draws inspiration from and acknowledges the following open-source projects:
1. [embedded-storage](https://github.com/rust-embedded-community/embedded-storage)
2. [embedded-nal](https://github.com/rust-embedded-community/embedded-nal)
