# libiot

A Rust SDK that enables any IoT device to securely connect to the cloud, manage data, and interact with cloud services.

## Project Structure

The project is organized as follows:

```
.
├── applications
│   └── esp32c3-demo
├── benches
│   └── network
│       └── application
│           └── mqtt
├── src
│   ├── network
│   │   ├── application
│   │   │   ├── coap
│   │   │   ├── http
│   │   │   ├── mcp
│   │   │   ├── mqtt
│   │   │   └── websocket
│   │   └── transport
│   ├── ota
│   ├── storage
│   └── system
├── tests
│   ├── network
│   │   ├── application
│   │   │   ├── http
│   │   │   └── mqtt
│   │   └── transport
│   └── storage
├── .env
├── Cargo.toml
└── README.md
```

## Usage

### Build Commands

| Command/Alias              | Description                    |
| -------------------------- | ------------------------------ |
| `cargo build`              | Build `libiot`                 |
| `cargo build-esp32c3-demo` | Build the `esp32c3-demo` app   |

### Benchmark Commands

| Command/Alias   | Description                          |
| --------------- | ------------------------------------ |
| `cargo bench`   | Run all benchmark tests for `libiot` |

### Test Commands

| Command/Alias | Description            |
| ------------- | ---------------------- |
| `cargo test`  | Run tests for `libiot` |

## Tech Stack

- **Rust**: The core programming language.
- **`#![no_std]`**: Designed for embedded systems without a standard library.
- **`futures`**: Used for testing asynchronous code.

## Acknowledgments

This library draws inspiration from and acknowledges the following open-source projects:
1. [embedded-storage](https://github.com/rust-embedded-community/embedded-storage)
2. [embedded-nal](https://github.com/rust-embedded-community/embedded-nal)
