# API Gateway Docker Setup

## Quick Start

### Start All Services (including Gateway)
```bash
cd docker
docker compose up
```

This starts:
- PostgreSQL Database
- gRPC Server
- REST API Server
- API Gateway Client
- CLI Client

### Start Only Gateway
```bash
cd docker
docker compose up gateway
```

The gateway will automatically start its dependency (gRPC server).

## Connection Details

**Gateway Service**
- Host: `localhost` (from your machine)
- Port: `8086`
- Network: `filevault-net` (for Docker containers)
- TLS Certificates: Mounted from `./pki/`

## Usage Examples

### From Your Machine (CLI)
```bash
cd api-gateway
go run main.go -host localhost -port 8086
```

### From Docker Container
```bash
docker exec -it filevault-gateway ./filedrop -host 0.0.0.0 -port 8086
```

## Docker Image

**Image**: `filevault-gateway:latest`
- **Size**: 25.6 MB
- **Base**: Alpine Linux
- **Go Version**: 1.26

## Architecture

```
┌─────────────────────────────────────────────┐
│          Docker Network (filevault-net)     │
│                                             │
│  ┌──────────────┐      ┌──────────────┐   │
│  │  Gateway     │      │  gRPC Server │   │
│  │  Port 8086   │◄────►│  Port 8083   │   │
│  │  (Client)    │      │  (Server)    │   │
│  └──────────────┘      └──────┬───────┘   │
│                                 │          │
│                        ┌────────▼────────┐ │
│                        │  PostgreSQL DB  │ │
│                        │  Port 5433      │ │
│                        └─────────────────┘ │
└─────────────────────────────────────────────┘
```

## Files

- **docker/Dockerfile.gateway**: Build configuration for the Go client
- **docker/docker-compose.yml**: Updated with gateway service
- **api-gateway/go.mod**: Updated Go version requirement (1.26)

## Commands

### Build Image Manually
```bash
docker build -f docker/Dockerfile.gateway -t filevault-gateway:latest .
```

### View Gateway Logs
```bash
docker compose logs gateway -f
```

### Check Gateway Status
```bash
docker compose ps gateway
```

### Shell into Gateway Container
```bash
docker exec -it filevault-gateway sh
```

## Health Check

The gateway service includes automatic health checks:
- **Test**: Connectivity to port 8086 using `netcat`
- **Interval**: Every 10 seconds
- **Retries**: 3 attempts before marking unhealthy
- **Startup**: 5-second grace period

View health status:
```bash
docker compose ps | grep gateway
```

## Certificates

TLS certificates are automatically mounted from `./pki/` as read-only volumes. This ensures:
- Secure communication with gRPC server
- Certificate verification
- No certificate modification in container

## Environment Variables

No environment variables needed for the gateway client. It inherits from docker-compose:
- Database secrets are only needed server-side
- JWT_SECRET is handled by server
- All client configuration is via command-line flags

## Troubleshooting

### Gateway won't start
```bash
# Check logs
docker compose logs gateway

# Verify gRPC server is healthy
docker compose ps grpc
```

### Can't connect to gateway
```bash
# Test port 8086
docker exec filevault-gateway nc -zv localhost 8086

# Check network connectivity
docker network inspect filevault-net
```

### Certificate errors
```bash
# Verify PKI directory exists
ls -la ./pki/

# Check mount in container
docker exec filevault-gateway ls -la /app/pki
```
