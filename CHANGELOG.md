# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2024-12-10

### Security & Compliance

- **SLSA Level 4**: Complete supply chain security implementation
  - Two-person review enforcement
  - Hermetic build environment
  - Build provenance attestation via SLSA GitHub Generator
  - `.github/workflows/slsa-release.yml` workflow

- **SBOM (Software Bill of Materials)**
  - 13 CycloneDX v1.5 SBOM files generated
  - `docs/SBOM_POLICY.md`: SBOM generation policy
  - Automated SBOM generation in CI/CD

### Code Quality

- **Error Handling Hardening**
  - Replace `unwrap()` with proper error handling in `core/src/engine.rs`
  - Replace `unwrap()` with mutex recovery in `core/src/admin_api.rs`
  - Graceful server binding errors

- **New Tests**: 15 REST API tests added
  - Response serialization tests
  - Handler tests (health, cluster status, nodes)
  - Error handling tests (not found, cancel)
  - Total test count: 88+ passing

### Quality Assurance

- **QA Report** (`docs/QA_REPORT.md`)
  - Code Coverage: 51.24% (1,161/2,266 lines)
  - Mutation Score: 40.1% (336/855 viable mutants)
  - Professional roadmap to 80% coverage

- **Coverage Report**: HTML report at `docs/coverage/tarpaulin-report.html`

### Documentation

- `docs/SLSA_COMPLIANCE.md`: Full SLSA Level 4 guide
- `docs/QA_REPORT.md`: Comprehensive quality metrics
- Updated `README.md` with verified benchmarks
- Fixed Issue #2: Remove incorrect `strip` option in PLUGIN_GUIDE.md

### Community

- First external contribution from @nickendwilestari-del
- Issue #2 resolved and closed

### Author

Wahyu Ardiansyah

---

## [0.2.1] - 2024-12-09

### Critical Bug Fixes

- **FFI Panic Safety**: All FFI functions wrapped with `catch_unwind` to prevent Rust panics from crashing Python
- **io_uring Graceful Degradation**: Replace `todo!()` with proper error returns
- **Zombie Job Detection**: Implement heartbeat and timeout mechanisms for scheduler
- **Input Validation**: Add comprehensive validation module for security

### New Features

#### High-Performance DataLoader
- `zenith-runtime-cpu/src/dataloader.rs`: Zero-copy Arrow/Parquet/CSV loading
- Batch iteration with caching for small datasets
- Automatic format detection

#### S3 Object Storage Adapter
- `zenith-runtime-cpu/src/s3.rs`: S3 configuration and streaming interface
- URI parsing (s3://bucket/key format)
- MinIO/LocalStack compatibility

#### Benchmark Suite
- `bench/`: Complete reproducible benchmark framework
- PyTorch DataLoader baseline comparison
- Zenith performance benchmarks
- Dataset generator for synthetic workloads

#### Clean Python API
- `zenith.load()`: One-line data loading
- `zenith.DataLoader`: Batch iteration
- `@zenith.job()`: Job scheduling decorator  
- `zenith.submit()`: Job submission

### Performance Results

| Metric        | Value                      |
|---------------|----------------------------|
| Throughput    | 1,351,591 samples/sec      |
| Latency p50   | 0.044 ms                   |
| Latency p99   | 0.074 ms                   |
| Improvement   | 4.2x vs streaming baseline |

### Documentation

- `docs/IMPLEMENTATION.md`: Comprehensive technical documentation
- `docs/KNOWN_ISSUES.md`: Issue tracking
- `docs/ROADMAP_v2.md`: Development roadmap
- `bench/README.md`: Benchmark reproducibility guide
- `bench/reports/BENCHMARK_REPORT.md`: Performance report

### Tests

- **52 tests passing** in zenith-runtime-cpu
- **5 tests passing** in zenith-core
- All critical paths covered

### Author

Wahyu Ardiansyah

---

## [0.2.0] - 2025-12-08

### Major Features

#### Phase 4: Zenith Turbo Engine
- **SIMD Processing** (`zenith-runtime-cpu/src/turbo/simd.rs`)
  - Feature detection (AVX2, AVX-512, NEON, SSE4)
  - Vectorized normalize, sum, mean, variance
  - Activation functions: ReLU, Sigmoid, Softmax
  - Batch matrix-vector multiply

- **Async Prefetch Pipeline** (`zenith-runtime-cpu/src/turbo/prefetch.rs`)
  - Zero-latency data loading
  - Thread-safe producer/consumer queues
  - Buffer recycling
  - Statistics tracking

- **Mixed Precision Engine** (`zenith-runtime-cpu/src/turbo/precision.rs`)
  - Float16 (FP16) support
  - BFloat16 (BF16) support
  - Dynamic loss scaling
  - Precision converter

- **ONNX Integration** (`zenith-runtime-cpu/src/turbo/onnx.rs`)
  - Execution providers (CPU, CUDA, TensorRT)
  - Session configuration
  - Model converter helpers

#### Phase 5: GPU Acceleration
- **CUDA Runtime** (`zenith-runtime-gpu/src/cuda.rs`)
  - Device management and properties
  - Memory allocation
  - Stream management
  - Kernel launch configuration

- **TensorRT Integration** (`zenith-runtime-gpu/src/tensorrt.rs`)
  - Engine building from ONNX
  - FP16/INT8 precision modes
  - Execution context
  - Optimization profiles

- **Multi-GPU Support** (`zenith-runtime-gpu/src/multigpu.rs`)
  - Topology discovery
  - NCCL-style collective operations
  - Data/Model/Pipeline parallelism
  - DataParallelTrainer

### Performance Targets

| Configuration | Throughput       | Speedup |
|---------------|------------------|---------|
| CPU (baseline)| 28K samples/sec  | 1x      |
| GPU FP32      | 500K samples/sec | 18x     |
| GPU FP16      | 1M samples/sec   | 36x     |
| TensorRT FP16 | 2-5M samples/sec | 100x    |
| TensorRT INT8 | 5-10M samples/sec| 350x    |

### Testing

- **Total Tests**: 73 passing (up from 41)
- **Turbo Engine**: 18 new tests
- **GPU Runtime**: 14 new tests

###  New Files

```
zenith-runtime-cpu/src/turbo/
├── mod.rs              # TurboEngine core
├── simd.rs             # SIMD operations
├── prefetch.rs         # Async prefetching
├── precision.rs        # Mixed precision
└── onnx.rs             # ONNX integration

zenith-runtime-gpu/src/
├── cuda.rs             # CUDA runtime
├── tensorrt.rs         # TensorRT
└── multigpu.rs         # Multi-GPU/NCCL

docs/
├── GPU_ACCELERATION.md # GPU guide
└── FREE_SOFTWARE.md    # License info
```

### Documentation

- GPU Acceleration Guide with API reference
- Community testing program
- Hardware sponsor opportunities
- All software confirmed FREE ($0)

### Status

GPU features are:
- [OK] Implemented based on official NVIDIA documentation
- [OK] Unit tested with mock implementations
- [!] Awaiting community validation on real hardware

---

## [0.1.1] - 2025-12-07

### New Features

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

### Performance

| Component | Metric | Value |
|-----------|--------|-------|
| Ring Buffer (SPSC) | Throughput | **43.16 M ops/sec** |
| Memory Pool | 1000 iterations | **32.69 ms** |
| Async File I/O | 1 MB read/write | **< 5 ms** |
| Telemetry | 10K events | **191 µs** |

### Testing

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

###  New Files

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

### Fixed

- Cleaned up all compilation warnings
- Fixed doc-test examples for all tests to pass
- Corrected API field mismatches with struct definitions

### Documentation

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
