# FileVault

<p align="center">
	<a href="#">
		<img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust">
	</a>
	<a href="#">
		<img src="https://img.shields.io/badge/protocol-TCP-blue.svg" alt="TCP">
	</a>
	<a href="#">
		<img src="https://img.shields.io/badge/integrity-SHA--256-brightgreen.svg" alt="SHA-256">
	</a>
	<a href="#">
		<img src="https://img.shields.io/badge/tests-Cargo_Test-success.svg" alt="Tests">
	</a>
	<a href="#">
		<img src="https://img.shields.io/badge/status-Academic_Project-lightgrey.svg" alt="Status">
	</a>
</p>

A lightweight Rust-based file transfer application using a TCP client/server architecture, with built-in SHA-256 integrity verification and transfer history logging.

## Table of Contents

1. Overview
2. Key Features
3. Project Structure
4. Requirements
5. Setup
6. Usage
7. Testing
8. Runtime Output
9. Troubleshooting
10. License

## Overview

FileVault provides a simple but robust way to send files over TCP.

Core capabilities:

- Run a TCP server that receives file uploads.
- Send files from a client to the server.
- Use an interactive mode for multiple transfer commands.
- Validate file integrity with SHA-256 checksums.
- Track transfer history in log files.

## Key Features

- Binary transfer protocol for filename and payload metadata.
- End-to-end integrity check (SHA-256 hash comparison).
- Dedicated error types for networking, file I/O, and protocol issues.
- Interactive CLI mode with reconnect and inline help.
- Isolated test runtime directories.
- Unit and integration tests for critical modules.

## Project Structure

Main source code is located in [file_vault](file_vault):

- [file_vault/src/main.rs](file_vault/src/main.rs): CLI entry point and runtime mode routing.
- [file_vault/src/server.rs](file_vault/src/server.rs): TCP listener, file receiving, hash validation, and logging.
- [file_vault/src/customer.rs](file_vault/src/customer.rs): TCP client connection and file sending logic.
- [file_vault/src/protocole.rs](file_vault/src/protocole.rs): packet serialization/deserialization.
- [file_vault/src/hashing.rs](file_vault/src/hashing.rs): hashing helper for streamed payload writes.
- [file_vault/src/cli_helpers.rs](file_vault/src/cli_helpers.rs): argument parsing and CLI helpers.
- [file_vault/tests](file_vault/tests): test suite.

## Requirements

- Rust toolchain (rustc + cargo)
- Linux, macOS, or Windows
- Local networking access for host/port communication

Check your toolchain:

```bash
cargo --version
```

## Setup

From the repository root:

```bash
cd file_vault
cargo build
```

Optional release build:

```bash
cargo build --release
```

## Usage

Run all commands from [file_vault](file_vault).

### 1. Start Server Only

```bash
cargo run -- server <host> <port>
```

Example:

```bash
cargo run -- server ::1 8080
```

### 2. Interactive Client/Server Mode

This mode starts listening and opens an interactive prompt:

```bash
cargo run -- <host> <port>
```

Example:

```bash
cargo run -- ::1 8080
```

Available interactive commands:

- `POST <filename> [--root <directory>]`
- `reconnect`
- `help`
- `help prod`
- `exit`

### 3. Legacy Compatibility Mode

Send file:

```bash
cargo run -- filevault --methode POST <host> <port> --filename <file> [--root <directory>]
```

GET mode (interactive):

```bash
cargo run -- filevault methode GET <host> <port>
```

The parser also supports `customer` and `method/--method` aliases.

### 4. Test Runtime Mode

Use `--test` (also `--tests` or `--test-mode`) to force runtime output into test folders:

- [file_vault/tests/log_files](file_vault/tests/log_files)
- [file_vault/tests/server_files](file_vault/tests/server_files)

Example:

```bash
cargo run -- --test ::1 8080
```

## Testing

Run full test suite:

```bash
cargo test
```

Run a specific test file:

```bash
cargo test --test server_test
```

## Runtime Output

Standard mode directories:

- [file_vault/server_files](file_vault/server_files): received files.
- [file_vault/log_files/history.log](file_vault/log_files/history.log): transfer history.

Test mode directories:

- [file_vault/tests/server_files](file_vault/tests/server_files)
- [file_vault/tests/log_files/history.log](file_vault/tests/log_files/history.log)

History entries include timestamp, filename, and payload size in bytes.

## Troubleshooting

- Connection failure: verify host/port and confirm no other process is using the same port.
- File not found: verify filename and `--root` path.
- Hash mismatch: payload was incomplete or corrupted, retry transfer.
- No interactive response: run `help` in prompt to check accepted commands.

## License
