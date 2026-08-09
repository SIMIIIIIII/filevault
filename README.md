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

Main source code is located at the repository root:

- [src/main.rs](src/main.rs): CLI entry point and runtime mode routing.
- [src/server.rs](src/server.rs): TCP listener, file receiving, hash validation, and logging.
- [src/customer.rs](src/customer.rs): TCP client connection and file sending logic.
- [src/protocole.rs](src/protocole.rs): packet serialization/deserialization.
- [src/hashing.rs](src/hashing.rs): hashing helper for streamed payload writes.
- [src/cli_helpers.rs](src/cli_helpers.rs): argument parsing and CLI helpers.
- [tests](tests): test suite.

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
cargo build
```

Optional release build:

```bash
cargo build --release
```

## Usage

Run all commands from the repository root.

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

- [tests/log_files](tests/log_files)
- [tests/server_files](tests/server_files)

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

### Test Coverage

From the repository root, run test coverage with `cargo-llvm-cov`:

```bash
cargo llvm-cov
```

Show only the summary in the terminal:

```bash
cargo llvm-cov --summary-only
```

Generate an HTML coverage report (default location under `target`):

```bash
cargo llvm-cov --html
```

Generate an HTML coverage report in a versionable folder for GitHub:

```bash
cargo llvm-cov clean --workspace
cargo llvm-cov --html --output-dir docs/coverage
```

If a flaky test fails but you still want to publish the report artifact:

```bash
cargo llvm-cov --html --output-dir docs/coverage --ignore-run-fail
```

Coverage report folder in this repository:

- [docs/coverage](docs/coverage)
- [docs/coverage/html/index.html](docs/coverage/html/index.html)

For a clean coverage run in the default location:

```bash
cargo llvm-cov clean --workspace
cargo llvm-cov --html
```

Export LCOV output for CI tools:

```bash
cargo llvm-cov --lcov --output-path target/llvm-cov/lcov.info
```

If you enable GitHub Pages from the `docs` folder, the report is typically available at:

- `https://<github-username>.github.io/<repository-name>/coverage/html/`

### Benchmarking

Run the throughput benchmark from the repository root:

```bash
cargo bench --bench project_wothout_tokio
```

Generated benchmark artifacts are stored in [benches/results](benches/results):

- `project_wothout_tokio.csv`: latest CSV for tooling.
- `project_wothout_tokio_YYYYMMDD_HHMMSS.csv`: timestamped CSV snapshot.
- `project_wothout_tokio.md`: latest Markdown summary.
- `project_wothout_tokio_YYYYMMDD_HHMMSS.md`: timestamped Markdown summary.

Read the latest CSV in the console:

```bash
cargo run --bin bench_results_reader
```

Read a specific benchmark CSV:

```bash
cargo run --bin bench_results_reader -- benches/results/project_wothout_tokio_YYYYMMDD_HHMMSS.csv
```

## Runtime Output

Standard mode directories:

- [server_files](server_files): received files.
- [log_files/history.log](log_files/history.log): transfer history.

Test mode directories:

- [tests/server_files](tests/server_files)
- [tests/log_files/history.log](tests/log_files/history.log)

History entries include timestamp, filename, and payload size in bytes.

## Troubleshooting

- Connection failure: verify host/port and confirm no other process is using the same port.
- File not found: verify filename and `--root` path.
- Hash mismatch: payload was incomplete or corrupted, retry transfer.
- No interactive response: run `help` in prompt to check accepted commands.

## License
