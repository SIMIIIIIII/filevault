.PHONY: help build build-release run-server run-api test test-api test-server coverage bench \
	db-up db-down db-wait migrate sqlx-prepare docker-up docker-down docker-build docker-logs clean fmt lint

help:
	@echo "Available targets:"
	@echo "  build           - cargo build (debug)"
	@echo "  build-release   - cargo build --release"
	@echo "  run-server      - run TCP server (HOST=:: PORT=8080)"
	@echo "  run-api         - run REST API (HOST=:: PORT=8081)"
	@echo "  test            - run full test suite"
	@echo "  test-api        - run tests/api_test.rs only"
	@echo "  test-server     - run tests/server_test.rs only"
	@echo "  coverage        - generate HTML coverage report (docs/coverage)"
	@echo "  bench           - run throughput benchmark"
	@echo "  db-up           - start db container and wait until healthy"
	@echo "  db-down         - stop db container"
	@echo "  migrate         - run sqlx migrations"
	@echo "  sqlx-prepare    - regenerate .sqlx offline query cache"
	@echo "  docker-build    - build all docker compose images"
	@echo "  docker-up       - build and start the full docker compose stack"
	@echo "  docker-down     - stop the docker compose stack"
	@echo "  docker-logs     - tail logs of all docker compose services"
	@echo "  fmt             - cargo fmt"
	@echo "  lint            - cargo clippy"
	@echo "  clean           - cargo clean"

HOST ?= ::
PORT ?= 8080
API_PORT ?= 8081

build:
	cargo build

build-release:
	cargo build --release

run-server: db-wait
	set -a; . ./.env; set +a; cargo run -- server $(HOST) $(PORT)

run-api: db-wait
	set -a; . ./.env; set +a; cargo run -- api $(HOST) $(API_PORT)

test: db-wait
	set -a; . ./.env; set +a; cargo test

test-api: db-wait
	set -a; . ./.env; set +a; cargo test --test api_test

test-server: db-wait
	set -a; . ./.env; set +a; cargo test --test server_test

coverage: db-wait
	set -a; . ./.env; set +a; \
	cargo llvm-cov clean --workspace && \
	cargo llvm-cov --html --output-dir docs/coverage

bench:
	cargo bench --bench project_wothout_tokio

db-up:
	docker compose up -d --wait db

# waits for postgres to accept connections before running commands that need it
db-wait: db-up
	until docker exec filevault-db pg_isready -U filevault -d filevault >/dev/null 2>&1; do sleep 1; done

db-down:
	docker compose stop db

migrate: db-wait
	set -a; . ./.env; set +a; sqlx migrate run

sqlx-prepare: db-wait
	set -a; . ./.env; set +a; cargo sqlx prepare -- --all-targets

docker-build:
	docker compose build

docker-up:
	docker compose up -d --build

docker-down:
	docker compose down

docker-logs:
	docker compose logs -f

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets

clean:
	cargo clean
