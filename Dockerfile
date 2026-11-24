# Build stage
FROM rust:latest as builder

RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    cmake \
    libcurl4-openssl-dev \
    libsasl2-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY .sqlx ./.sqlx
COPY Cargo.toml Cargo.lock build.rs ./
COPY proto ./proto
COPY src ./src
COPY filevault-cli ./filevault-cli
COPY benches ./benches
COPY tests ./tests

# SQLx uses cached database schema; set environment variable to enable offline mode
ENV SQLX_OFFLINE=true

RUN cargo build --release --package file_vault --bin server

# Runtime stage
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/server /usr/local/bin/filevault

RUN mkdir -p /data

EXPOSE 8443

ENTRYPOINT ["/usr/local/bin/filevault"]
CMD ["server", "0.0.0.0", "8443"]
