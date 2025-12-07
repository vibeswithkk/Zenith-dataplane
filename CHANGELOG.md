# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-12-07

### 🚀 New Features

#### Phase 1: Core Runtime Enhancements
- **Prometheus Metrics Export** (`zenith-runtime-cpu/src/metrics.rs`)
  - HTTP endpoint `/metrics` for Prometheus scraping
  - `/health` endpoint for liveness checks
  - All telemetry metrics exposed in Prometheus format

- **Scheduler REST API** (`zenith-scheduler/src/api/rest.rs`)
  - `POST /api/v1/jobs` - Submit job
  - `GET /api/v1/jobs/:id` - Get job status
  - `DELETE /api/v1/jobs/:id` - Cancel job
  - `GET /api/v1/cluster/status` - Cluster overview
  - `GET /api/v1/nodes` - List nodes

- **Scheduler gRPC API** (`zenith-scheduler/src/api/grpc.rs`)
  - SubmitJob, GetJobStatus, CancelJob, GetClusterStatus

- **Node Agent** (`zenith-scheduler/src/agent.rs`)
  - GPU discovery via nvidia-smi
  - CPU/memory topology discovery
  - NUMA node detection
  - Heartbeat mechanism

#### Phase 2: Advanced Implementation
- **Full io_uring Implementation** (`zenith-runtime-cpu/src/uring.rs`)
  - Submission/completion queue management
  - Read/Write/Fsync operations
  - Vectored I/O support
  - Thread-safe with Mutex-protected ring

- **High-Performance Memory Pool** (`zenith-runtime-cpu/src/pool.rs`)
  - Slab allocation for fixed-size buffers
  - Pool statistics and high-water mark tracking
  - Zero-copy buffer access

- **NVML-like GPU Management** (`zenith-runtime-gpu/src/nvml.rs`)
  - GPU discovery via nvidia-smi
  - Memory, utilization, clock, temperature monitoring
  - PCIe and NVLink status
  - Power limit control

- **State Persistence** (`zenith-scheduler/src/state.rs`)
  - Job and node state persistence
  - JSON file-based storage
  - Job state transitions

#### Phase 3: Production Implementation
- **Health Check System** (`zenith-runtime-cpu/src/health.rs`)
  - Liveness/readiness probes
  - Kubernetes-compatible health endpoints
  - Memory and disk health checks

- **Circuit Breaker Pattern** (`zenith-runtime-cpu/src/circuit_breaker.rs`)
  - Fault tolerance implementation
  - Configurable thresholds
  - Statistics tracking

- **Kubernetes Integration**
  - Complete Helm chart (`deploy/helm/zenith/`)
  - Multi-stage Dockerfile
  - Service definitions

- **CI/CD Pipeline** (`.github/workflows/ci.yml`)
  - Build and test
  - Code coverage with tarpaulin
  - Security audit
  - Documentation build
  - Benchmarks
  - Docker build test

- **API Documentation** (`docs/api/openapi.yaml`)
  - OpenAPI 3.0 specification
  - All REST endpoints documented
  - Request/response schemas

### 📈 Performance

| Component | Metric | Value |
|-----------|--------|-------|
| Ring Buffer (SPSC) | Throughput | **43.16 M ops/sec** |
| Memory Pool | 1000 iterations | **32.69 ms** |
| Async File I/O | 1 MB read/write | **< 5 ms** |
| Telemetry | 10K events | **191 µs** |

### 🧪 Testing

- **Unit Tests**: 41 passing (up from 20)
- **Integration Tests**: 6 passing
- **Doc Tests**: 2 passing

New test modules:
- `pool::tests` (4 tests)
- `uring::tests` (2 tests)
- `nvml::tests` (2 tests)
- `state::tests` (2 tests)
- `health::tests` (2 tests)
- `circuit_breaker::tests` (3 tests)

### 📁 New Files

```
zenith-runtime-cpu/src/
├── metrics.rs          # Prometheus export
├── pool.rs             # Memory pool
├── uring.rs            # io_uring implementation
├── health.rs           # Health checks
├── circuit_breaker.rs  # Fault tolerance
└── tests/integration.rs # Integration tests

zenith-runtime-gpu/src/
└── nvml.rs             # NVML-like GPU management

zenith-scheduler/src/
├── agent.rs            # Node agent
├── api/grpc.rs         # gRPC API
├── api/rest.rs         # REST API
└── state.rs            # State persistence

deploy/helm/zenith/     # Kubernetes Helm chart
docs/api/openapi.yaml   # OpenAPI specification
.github/workflows/ci.yml # CI/CD pipeline
Dockerfile              # Production Docker build
ROADMAP.md              # Development roadmap
```

### 🔧 Fixed

- Cleaned up all compilation warnings
- Fixed doc-test examples for all tests to pass
- Corrected API field mismatches with struct definitions

### 📚 Documentation

- Added comprehensive ROADMAP.md with sponsorship tiers
- Added OpenAPI 3.0 specification
- Updated ARCHITECTURE.md
- Added validation scripts

---

## [0.1.0] - 2025-12-06

### Initial Release

- Core CPU runtime with lock-free ring buffers
- NUMA-aware memory allocation
- GPU runtime abstraction layer
- Job scheduler with gang scheduling
- Python SDK for PyTorch/TensorFlow
- Basic documentation

---

## Future Releases

### [0.2.0] - Planned Q1 2025
- Native CUDA kernel integration
- NCCL collective operations
- Production Kubernetes testing
- Performance optimization

### [0.3.0] - Planned Q2 2025
- RDMA/InfiniBand support
- NVMe-oF integration
- Triton/TVM kernel support

### [1.0.0] - Planned Q3 2025
- Kubernetes Operator
- Web Dashboard
- Multi-tenancy support
- Enterprise features
