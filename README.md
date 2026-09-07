# FileVault

<p align="center">
  <a href="#">
    <img src="https://img.shields.io/badge/Rust-1.88%2B-orange?logo=rust" alt="Rust 1.88+" />
  </a>
  <a href="#">
    <img src="https://img.shields.io/badge/Protocol-TCP-blue" alt="TCP protocol" />
  </a>
  <a href="#">
    <img src="https://img.shields.io/badge/Storage-PostgreSQL-336791?logo=postgresql" alt="PostgreSQL" />
  </a>
  <a href="#">
    <img src="https://img.shields.io/badge/API-Axum%20%2B%20JWT-8b5cf6?logo=axum" alt="Axum + JWT" />
  </a>
  <a href="#">
    <img src="https://img.shields.io/badge/Container-Docker-2496ED?logo=docker" alt="Docker" />
  </a>
  <a href="#">
    <img src="https://img.shields.io/badge/Integrity-SHA--256-brightgreen" alt="SHA-256 integrity" />
  </a>
  <a href="#">
    <img src="https://img.shields.io/badge/Tests-Cargo_Test-success" alt="Cargo tests" />
  </a>
</p>

FileVault is a Rust project for transferring files with a lightweight TCP protocol and a JWT-authenticated REST API. It stores file metadata in PostgreSQL, verifies payload integrity with SHA-256, and exposes both a server-side upload flow and a CLI client.

## Table of contents

1. Overview
2. Features
3. Project structure
4. Requirements
5. Quick start
6. Environment configuration
7. Database setup
8. Running the TCP server
9. Running the REST API
10. Using the CLI
11. Docker compose
12. Testing

## Overview

The project contains two main interaction layers:

- a TCP server/client mode for direct file transfer
- an Axum REST API for user authentication, file management, and statistics

The data model persists file metadata such as name, size, uploader, and timestamps in PostgreSQL. Each uploaded file is also saved on disk and validated through SHA-256 checks before being accepted.

## Features

- TCP file upload between client and server
- SHA-256 integrity verification on received payloads
- PostgreSQL-backed file metadata storage
- JWT authentication for the HTTP API
- CLI client for login, upload, download, list, and stats
- Interactive legacy mode for direct client/server transfer testing
- Dockerized local stack for database, API, server, and client
- Integration test suite for API and server behavior

## Project structure

```text
.
├── Cargo.toml
├── docker-compose.yml
├── Dockerfile.api
├── Dockerfile.cli
├── Dockerfile.customer
├── Dockerfile.server
├── Makefile
├── README.md
├── migrations/
├── src/
│   ├── api/
│   ├── bin/
│   ├── main.rs
│   ├── server.rs
│   ├── customer.rs
│   ├── db.rs
│   ├── files.rs
│   ├── cli_helpers.rs
│   ├── protocole.rs
│   ├── hashing.rs
│   └── ...
├── filevault-cli/
│   ├── Cargo.toml
│   └── src/
├── tests/
├── log_files/
├── server_files/
└── docs/
```

Main modules:

- [src/main.rs](src/main.rs): entry point for the main binary and runtime mode selection
- [src/server.rs](src/server.rs): TCP listener and file ingestion logic
- [src/customer.rs](src/customer.rs): TCP client logic for sending files
- [src/api](src/api): HTTP routes and handlers for the REST API
- [src/db.rs](src/db.rs): PostgreSQL connection and database queries
- [src/cli_helpers.rs](src/cli_helpers.rs): parsing utilities and runtime mode helpers
- [filevault-cli/src/main.rs](filevault-cli/src/main.rs): command-line HTTP client for the API
- [migrations](migrations): SQL migration scripts
- [tests](tests): project test suite

## Requirements

- Rust 1.88 or newer
- Cargo
- Docker and Docker Compose
- PostgreSQL 16 (provided by the Docker stack)
- sqlx-cli for migrations and SQLx metadata refresh

Check the toolchain:

```bash
cargo --version
```

Install the SQLx CLI if you need to run migrations from the host machine:

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

## Quick start

1. Create a `.env` file in the project root.
2. Start PostgreSQL.
3. Run the migrations.
4. Start the server or API.
5. Use the CLI or the direct TCP client.

Example for a local environment:

```bash
cp .env.example .env
```

If there is no `.env.example`, create the file manually with:

```env
DATABASE_URL=postgresql://filevault:changeme@localhost:5433/filevault
DATABASE_URL_TEST=postgresql://filevault:changeme@localhost:5433/filevaulttest
JWT_SECRET=replace_with_a_secure_random_value
JWT_SECRET_TEST=replace_with_another_secure_random_value
```

## Environment configuration

The project expects a PostgreSQL database and a JWT secret for the API runtime.

Generate a JWT secret with:

```bash
cargo run --bin key
```

This command produces a secure random string suitable for `JWT_SECRET`.

## Database setup

The project is configured to use PostgreSQL through Docker Compose.

Start only the database and wait for it to be healthy:

```bash
docker compose up -d --wait db
```

Then run migrations:

```bash
set -a
source .env
set +a
sqlx migrate run
```

If you change SQL queries or add a migration, regenerate the offline SQLx metadata:

```bash
set -a
source .env
set +a
cargo sqlx prepare -- --all-targets
```

## Running the TCP server

The main server binary runs the file transfer listener.

```bash
cargo run -- server ::1 8080
```

This starts a TCP server bound to the host and port specified.

The project also supports a direct interactive mode:

```bash
cargo run -- ::1 8080
```

Available interactive commands:

- `POST <filename> [--root <directory>]`
- `reconnect`
- `help`
- `help prod`
- `exit`

Legacy compatibility mode is also still available:

```bash
cargo run -- filevault --methode POST ::1 8080 --filename README.md
cargo run -- filevault methode GET ::1 8080
```

## Running the REST API

Start the Axum API with:

```bash
set -a
source .env
set +a
cargo run -- api ::1 8081
```

The API exposes these routes:

- `GET /health`
- `POST /auth/register`
- `POST /auth/login`
- `GET /files`
- `POST /files`
- `GET /files/:id`
- `DELETE /files/:id`
- `GET /stats`

The protected endpoints require a bearer token in the `Authorization` header:

```http
Authorization: Bearer <token>
```

The token is returned by `/auth/login`.

## Using the CLI

The workspace includes a dedicated HTTP client under [filevault-cli](filevault-cli).

### Login

```bash
cargo run -p filevault-cli -- --server http://localhost:8081 login --email user@example.com --mot_de_passe secret
```

This saves the JWT token in the local CLI configuration.

### Upload a file

```bash
cargo run -p filevault-cli -- --server http://localhost:8081 upload ./README.md
```

### Download a file

```bash
cargo run -p filevault-cli -- --server http://localhost:8081 download 1 --output ./downloaded.md
```

### List files

```bash
cargo run -p filevault-cli -- --server http://localhost:8081 list
```

### Show statistics

```bash
cargo run -p filevault-cli -- --server http://localhost:8081 stats
```

## Docker compose

The Compose file starts the full local stack:

- `db`: PostgreSQL 16
- `server`: TCP file server on port `8080`
- `api`: REST API on port `8081`
- `customer`: demo client sending `README.md`
- `cli`: example API client command

Start the stack:

```bash
docker compose up --build
```

Or start only the required services if you are developing iteratively:

```bash
docker compose up -d db api cli
```

Stop everything:

```bash
docker compose down
```

The API service reads `JWT_SECRET` from the host environment or `.env` file.

## Testing

The project includes Rust tests for the API and server layers.

Run the full suite:

```bash
cargo test
```

Run a focused subset:

```bash
cargo test --test api_test
cargo test --test server_test
```

A useful helper target is also available from the Makefile:

```bash
make test
make test-api
make test-server
```

## Notes

The repository includes both the main application and a CLI workspace package. For backend-focused development, the main entry points are the server and API binaries. The file transfer protocol is intentionally simple and can be used for local networking tests and academic demonstrations.

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
