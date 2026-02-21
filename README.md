# RTP Fanout Server

A high-performance, lock-free RTP (Real-time Transport Protocol) fanout server with session-based gating for media streaming applications.

[![Build Status](https://github.com/ottopia-tech/rtp-fanout-server/workflows/CI/badge.svg)](https://github.com/ottopia-tech/rtp-fanout-server/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.83%2B-blue.svg)](https://www.rust-lang.org)

## Features

- **Lock-free Architecture**: Uses crossbeam lock-free data structures for maximum throughput
- **Session-based Gating**: Fine-grained control over media streams with per-session subscriber limits
- **High Performance**: Designed for 10,000+ concurrent sessions with 1000+ subscribers per session
- **Prometheus Metrics**: Built-in observability with packet counts, latency histograms, and session statistics
- **gRPC Control API**: Programmatic session management and subscriber control
- **Docker Support**: Ready-to-deploy container images
- **Rover Integration**: Compatible with Ottopia's media infrastructure

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    RTP Fanout Server                         │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐    │
│  │ UDP Socket   │──▶│ Packet Queue │──▶│ Fanout Engine│    │
│  └──────────────┘   │ (Lock-free)  │   └──────────────┘    │
│                     └──────────────┘          │             │
│                                               ▼             │
│                              ┌──────────────────────────┐   │
│                              │    Session Manager       │   │
│                              │  ┌────────────────────┐  │   │
│                              │  │ Session 1 (SSRC A) │  │   │
│                              │  │  - Subscribers     │  │   │
│                              │  │  - Packet counters │  │   │
│                              │  └────────────────────┘  │   │
│                              │  ┌────────────────────┐  │   │
│                              │  │ Session 2 (SSRC B) │  │   │
│                              │  │  - Subscribers     │  │   │
│                              │  │  - Packet counters │  │   │
│                              │  └────────────────────┘  │   │
│                              └──────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │   gRPC Control API   │
                    └──────────────────────┘
```

### Key Components

1. **UDP Socket**: Receives RTP packets on the configured port
2. **Packet Queue**: Lock-free SegQueue for buffering incoming packets
3. **Fanout Engine**: Processes packets and distributes to subscribers
4. **Session Manager**: Manages media sessions indexed by SSRC
5. **gRPC API**: Control plane for session lifecycle management

## Build Instructions

### Prerequisites

- Rust 1.83+ (install via [rustup](https://rustup.rs/))
- Protocol Buffers compiler (`protoc`)
- Docker (optional, for container builds)

### Local Build

```bash
# Clone the repository
git clone https://github.com/ottopia-tech/rtp-fanout-server.git
cd rtp-fanout-server

# Build release binary
cargo build --release

# Run tests
cargo test

# Build with all features
cargo build --release --all-features
```

### Docker Build

```bash
# Build Docker image
docker build -t rtp-fanout-server:latest -f build/Dockerfile .

# Run container
docker run -p 5004:5004/udp -p 9090:9090 rtp-fanout-server:latest
```

## Configuration Guide

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RTP_FANOUT__BIND_ADDRESS` | `0.0.0.0:5004` | UDP listen address for RTP |
| `RTP_FANOUT__MAX_SESSIONS` | `10000` | Maximum concurrent sessions |
| `RTP_FANOUT__MAX_FANOUT_PER_SESSION` | `1000` | Max subscribers per session |
| `RTP_FANOUT__BUFFER_SIZE` | `65536` | Internal packet buffer size |
| `RTP_FANOUT__SESSION_TIMEOUT_SECS` | `300` | Session idle timeout |
| `RTP_FANOUT__ENABLE_METRICS` | `true` | Enable Prometheus metrics |
| `RTP_FANOUT__METRICS_BIND_ADDRESS` | `0.0.0.0:9090` | Metrics HTTP endpoint |

### Configuration File

Create `config/server.toml`:

```toml
bind_address = "0.0.0.0:5004"
max_sessions = 10000
max_fanout_per_session = 1000
buffer_size = 65536
session_timeout_secs = 300
enable_metrics = true
metrics_bind_address = "0.0.0.0:9090"
```

## API Documentation

### gRPC Control API

The server exposes a gRPC API on port 50051 for session management.

#### Service: SessionService

```protobuf
service SessionService {
  rpc CreateSession(CreateSessionRequest) returns (SessionResponse);
  rpc GetSession(GetSessionRequest) returns (SessionResponse);
  rpc DeleteSession(DeleteSessionRequest) returns (google.protobuf.Empty);
  rpc ListSessions(ListSessionsRequest) returns (ListSessionsResponse);
  rpc AddSubscriber(AddSubscriberRequest) returns (google.protobuf.Empty);
  rpc RemoveSubscriber(RemoveSubscriberRequest) returns (google.protobuf.Empty);
  rpc GetSessionStats(GetSessionStatsRequest) returns (SessionStatsResponse);
}
```

#### Example: Create Session

```bash
grpcurl -plaintext -d '{
  "source_address": "192.168.1.100:5004",
  "ssrc": 1234567890,
  "media_type": "video"
}' localhost:50051 rtpfanout.SessionService/CreateSession
```

#### Example: Add Subscriber

```bash
grpcurl -plaintext -d '{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "subscriber_address": "192.168.1.101:6000"
}' localhost:50051 rtpfanout.SessionService/AddSubscriber
```

### Metrics Endpoints

Prometheus metrics available at `http://localhost:9090/metrics`:

- `rtp_packets_received_total` - Total RTP packets received
- `rtp_packets_sent_total` - Total RTP packets sent to subscribers
- `rtp_bytes_received_total` - Total bytes received
- `fanout_latency_ms` - Histogram of fanout latency
- `active_sessions` - Gauge of currently active sessions
- `total_subscribers` - Gauge of total connected subscribers

## Deployment Guide

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rtp-fanout-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rtp-fanout-server
  template:
    metadata:
      labels:
        app: rtp-fanout-server
    spec:
      containers:
      - name: server
        image: ottopia-tech/rtp-fanout-server:latest
        ports:
        - containerPort: 5004
          protocol: UDP
        - containerPort: 9090
        env:
        - name: RTP_FANOUT__MAX_SESSIONS
          value: "10000"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
```

### Docker Compose

```yaml
version: '3.8'
services:
  rtp-fanout:
    image: ottopia-tech/rtp-fanout-server:latest
    ports:
      - "5004:5004/udp"
      - "9090:9090"
    environment:
      - RTP_FANOUT__MAX_SESSIONS=5000
      - RUST_LOG=info
    volumes:
      - ./config:/etc/rtp-fanout
```

### Performance Tuning

1. **Network Stack** (Linux):
   ```bash
   # Increase UDP buffer sizes
   sysctl -w net.core.rmem_max=134217728
   sysctl -w net.core.wmem_max=134217728
   ```

2. **CPU Affinity**: Pin server threads to specific cores for consistent latency

3. **NUMA Awareness**: Run on NUMA node local to network interface

## Rover Integration

The server integrates with Ottopia's Rover platform for autonomous vehicle media streaming:

- Accepts RTP streams from vehicle cameras
- Distributes to multiple consumers (recording, ML inference, monitoring)
- Session-based access control per vehicle
- Real-time metrics for QoS monitoring

## License

MIT License - See [LICENSE](LICENSE) for details.

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Support

For issues and feature requests, please use [GitHub Issues](https://github.com/ottopia-tech/rtp-fanout-server/issues).
