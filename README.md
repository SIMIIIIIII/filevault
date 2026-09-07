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
5. Database Setup
6. REST API
7. Docker Usage
8. Local Usage
9. Testing
10. Runtime Output
11. Troubleshooting
12. License

## Overview

FileVault provides a simple but robust way to send files over TCP.

Core capabilities:

- Run a TCP server that receives file uploads.
- Send files from a client to the server.
- Use an interactive mode for multiple transfer commands.
- Validate file integrity with SHA-256 checksums.
- Track transfer history in log files.
- Store received-file metadata in PostgreSQL.
- Expose a JWT-authenticated REST API (Axum) for registration, login, file upload/download, and statistics.

## Key Features

- Binary transfer protocol for filename and payload metadata.
- End-to-end integrity check (SHA-256 hash comparison).
- Dedicated error types for networking, file I/O, and protocol issues.
- Interactive CLI mode with reconnect and inline help.
- Isolated test runtime directories.
- Unit and integration tests for critical modules.
- Bearer-token authentication (JWT) for the REST API.

## Project Structure

Main source code is located at the repository root:

- [src/main.rs](src/main.rs): CLI entry point and runtime mode routing (`server`, `api`, legacy modes).
- [src/server.rs](src/server.rs): TCP listener, file receiving, hash validation, and logging.
- [src/customer.rs](src/customer.rs): TCP client connection and file sending logic.
- [src/protocole.rs](src/protocole.rs): packet serialization/deserialization.
- [src/hashing.rs](src/hashing.rs): hashing helper for streamed payload writes.
- [src/cli_helpers.rs](src/cli_helpers.rs): argument parsing and CLI helpers.
- [src/db.rs](src/db.rs): PostgreSQL connection and file metadata queries.
- [src/api](src/api): Axum REST API (routes, handlers, JWT auth middleware, app state).
- [migrations](migrations): SQL database migrations.
- [.sqlx](.sqlx): SQLx offline query metadata used for Docker builds.
- [tests](tests): test suite.

## Requirements

- Rust 1.88 or later (rustc + cargo)
- Docker and Docker Compose for the containerized setup
- `sqlx-cli` for running database migrations and regenerating SQLx metadata
- Linux, macOS, or Windows
- Local networking access for host/port communication

Check your toolchain:

```bash
cargo --version
```

Install the SQLx CLI when using the database from the host:

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

## Database Setup

Docker Compose starts PostgreSQL 16 with these development defaults:

- Host connection: `postgresql://filevault:changeme@localhost:5433/filevault`
- Container connection: `postgresql://filevault:changeme@db:5432/filevault`

Create a local `.env` file for commands run from the host:

```env
DATABASE_URL=postgresql://filevault:changeme@localhost:5433/filevault
DATABASE_URL_TEST=postgresql://filevault:changeme@localhost:5433/filevaulttest
JWT_SECRET=<a random secret>
JWT_SECRET_TEST=<a random secret for tests>
```

`JWT_SECRET` signs and validates the API's JWT tokens; generate one with the `key` binary:

```bash
cargo run --bin key
```

Start the database and apply migrations:

```bash
docker compose up -d --wait db
set -a
source .env
set +a
sqlx migrate run
```

The `sqlx::query!` and `sqlx::query_as!` macros validate SQL at compile time. After changing a query or a migration, refresh the committed offline metadata:

```bash
set -a
source .env
set +a
cargo sqlx prepare -- --all-targets
```

## REST API

The `api` binary mode exposes a JWT-authenticated REST API built with Axum. Locally:

```bash
set -a
source .env
set +a
cargo run -- api ::1 8081
```

Routes:

- `GET /health`: liveness check, no auth required.
- `POST /auth/register`: create a user (`email`, `username`, `password`, `fullname`).
- `POST /auth/login`: authenticate and receive a JWT (`{ "token": "..." }`).
- `GET /files`: list the authenticated user's files.
- `POST /files`: upload a file (multipart).
- `GET /files/:id`: download a file by id.
- `DELETE /files/:id`: delete a file by id.
- `GET /stats`: aggregate statistics (total files, bytes, users).

All routes under `/files` and `/stats` require an `Authorization: Bearer <token>` header obtained from `/auth/login`.

## Docker Usage

Build and start the application stack:

```bash
docker compose up --build
```

This starts four services: `db` (PostgreSQL), `server` (TCP file server on port `8080`), `api` (REST API on port `8081`), and `customer` (a one-shot client sending `README.md` to `server`). Docker supplies `DATABASE_URL` automatically for `server`/`api` and waits for PostgreSQL to become healthy; `JWT_SECRET` for the `api` service is read from the host environment/`.env` file. Stop the stack with:

```bash
docker compose down
```

## Local Usage

From the repository root:

```bash
set -a
source .env
set +a
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
- `DATABASE_URL doit être définie`: load and export `.env` with `set -a; source .env; set +a` before running Cargo commands.
- `set DATABASE_URL to use query macros online`: start the database and run `cargo sqlx prepare -- --all-targets`; Docker builds use the generated `.sqlx` cache.
- `error communicating with database ... EOF`: confirm that PostgreSQL is healthy with `docker compose ps` and use port `5433` from the host, not `5432`.
- `[ERROR]: JWT_SECRET wasn't defined`: export `JWT_SECRET` (host) or set it in `docker-compose.yml`'s `api` service before starting the API.
- `SQLX_OFFLINE=true but there is no cached data for this query`: a query changed but `.sqlx` wasn't refreshed; run `cargo sqlx prepare` against a live database and commit the updated `.sqlx` folder.
- File not found: verify filename and `--root` path.
- Hash mismatch: payload was incomplete or corrupted, retry transfer.
- No interactive response: run `help` in prompt to check accepted commands.

## License
