# Zenith AI Infrastructure - Development Roadmap

<div align="center">

![Zenith Logo](https://img.shields.io/badge/Zenith-AI%20Infrastructure-blue?style=for-the-badge&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI+PHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0xMiAyTDEgMjFoMjJMMTIgMnoiLz48L3N2Zz4=)

**High-Performance Data Infrastructure for ML Training Pipelines**

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/Status-Active%20Development-green.svg)](#current-status)

</div>

---

## Table of Contents

- [Current Status](#-current-status)
- [What's Working (Production Ready)](#-whats-working-production-ready)
- [Needs Additional Testing](#-needs-additional-testing)
- [Features in Development](#-features-in-development)
- [Roadmap Timeline](#-roadmap-timeline)
- [Hardware Requirements](#-hardware-requirements)
- [How to Contribute](#-how-to-contribute)
- [Sponsorship Opportunities](#-sponsorship-opportunities)
- [Contact](#-contact)

---

## Current Status

| Metric | Value |
|--------|-------|
| **Lines of Code** | ~10,500+ |
| **Unit Tests** | 41 passing |
| **Integration Tests** | 6 passing |
| **Crates** | 5 |
| **Development Phase** | Phase 3 Complete |

---

## What's Working (Production Ready)

These components have been thoroughly tested and are ready for production use:

### Core Runtime Components

| Component | Description | Performance | Tests |
|-----------|-------------|-------------|-------|
| **SPSC Ring Buffer** | Lock-free single-producer single-consumer queue | **43M+ ops/sec** | [OK] 4 tests |
| **MPMC Ring Buffer** | Multi-producer multi-consumer queue | High throughput | [OK] Tested |
| **Memory Pool** | Slab-based memory allocation | Zero leaks | [OK] 4 tests |
| **NUMA Allocator** | NUMA-aware memory allocation | Optimized | [OK] 2 tests |
| **NUMA Topology Discovery** | Automatic system topology detection | < 1ms | [OK] 2 tests |
| **Telemetry Collector** | Real-time metrics collection | 10K events in 191µs | [OK] Tested |
| **Thread Pool** | CPU-pinned thread management | Per-core affinity | [OK] 2 tests |

### Scheduler Components

| Component | Description | Features | Tests |
|-----------|-------------|----------|-------|
| **Job State Machine** | Job lifecycle management | 7 states, transitions | [OK] 2 tests |
| **Gang Scheduler** | All-or-nothing resource allocation | Topology-aware | [OK] 1 test |
| **Node Registry** | Compute node management | GPU tracking | [OK] 3 tests |
| **State Persistence** | Durable job/node storage | JSON-based | [OK] 2 tests |

### Production Hardening

| Component | Description | Pattern | Tests |
|-----------|-------------|---------|-------|
| **Circuit Breaker** | Fault tolerance | Industry standard | [OK] 3 tests |
| **Health Checks** | Liveness/readiness probes | K8s compatible | [OK] 2 tests |
| **Prometheus Metrics** | Metrics export | `/metrics` endpoint | [OK] Ready |

---

## Needs Additional Testing

These components are implemented but require testing in real environments:

### 1. Kubernetes Deployment

| Item | Current Status | Testing Required |
|------|----------------|------------------|
| **Helm Chart** | [OK] Templates valid, YAML verified | Deploy to real K8s cluster |
| **Dockerfile** | [OK] Multi-stage build ready | Build image, run container |
| **Service Discovery** | [OK] Service definitions ready | Test with real Kubernetes DNS |
| **Horizontal Scaling** | [OK] HPA configuration ready | Load test with auto-scaling |

**How to test:**
```bash
# Install on Kubernetes cluster
helm install zenith ./deploy/helm/zenith --namespace zenith --create-namespace

# Verify pods are running
kubectl get pods -n zenith

# Test health endpoint
kubectl port-forward svc/zenith-scheduler 8080:8080 -n zenith
curl http://localhost:8080/health
```

### 2. REST API Server

| Endpoint | Method | Status | Testing Required |
|----------|--------|--------|------------------|
| `/api/v1/jobs` | POST | [OK] Implemented | Submit real jobs |
| `/api/v1/jobs/:id` | GET | [OK] Implemented | Query job status |
| `/api/v1/jobs/:id` | DELETE | [OK] Implemented | Cancel running jobs |
| `/api/v1/cluster/status` | GET | [OK] Implemented | Multi-node status |
| `/api/v1/nodes` | GET | [OK] Implemented | Node listing |
| `/health` | GET | [OK] Implemented | N/A (ready) |

**How to test:**
```bash
# Start the scheduler
cargo run -p zenith-scheduler

# Test endpoints
curl -X POST http://localhost:8080/api/v1/jobs \
  -H "Content-Type: application/json" \
  -d '{"name":"test-job","user_id":"user1","project_id":"proj1","command":"python","arguments":["train.py"]}'
```

### 3. gRPC API Server

| Service | Method | Status | Testing Required |
|---------|--------|--------|------------------|
| SchedulerService | SubmitJob | [OK] Implemented | gRPC client calls |
| SchedulerService | GetJobStatus | [OK] Implemented | Status queries |
| SchedulerService | CancelJob | [OK] Implemented | Cancellation flow |
| SchedulerService | GetClusterStatus | [OK] Implemented | Cluster overview |

**How to test:**
```bash
# Using grpcurl
grpcurl -plaintext localhost:50051 list
grpcurl -plaintext -d '{"name":"test"}' localhost:50051 zenith.Scheduler/SubmitJob
```

### 4. GPU Features

| Feature | Current Status | Hardware Required |
|---------|----------------|-------------------|
| **GPU Discovery** | nvidia-smi parsing | NVIDIA GPU + driver |
| **Memory Monitoring** | nvidia-smi based | Any NVIDIA GPU |
| **NVLink Detection** | Placeholder | Multi-GPU NVLink system |
| **GPU Topology** | Basic implementation | GPU server |

**How to test:**
```bash
# Requires NVIDIA GPU
nvidia-smi  # Verify GPU is available
cargo test -p zenith-runtime-gpu -- --nocapture
```

### 5. io_uring Async I/O

| Feature | Current Status | Requirements |
|---------|----------------|--------------|
| **Basic Operations** | [OK] Implemented | Linux kernel 5.1+ |
| **Submission Queue** | [OK] Mutex-protected | Modern Linux |
| **Completion Queue** | [OK] Implemented | io_uring support |
| **Registered Buffers** | Planned | Advanced use case |

**How to test:**
```bash
# Check kernel version (need 5.1+)
uname -r

# Run integration tests
cargo test --test integration -p zenith-runtime-cpu -- uring --nocapture
```

---

## Features in Development

These features are planned or have placeholder implementations:

### High Priority (Phase 4)

| Feature | Description | Effort | Sponsor Opportunity |
|---------|-------------|--------|---------------------|
| **CUDA Kernel Integration** | Native CUDA kernel management | 2-3 weeks | $5,000 |
| **NCCL Collective Operations** | All-reduce, broadcast, etc. | 2-3 weeks | $5,000 |
| **Node Agent -> Scheduler gRPC** | Real heartbeat/registration | 1 week | $2,000 |
| **Multi-node E2E Testing** | Distributed cluster testing | 1 week | $2,000 |

### Medium Priority (Phase 5)

| Feature | Description | Effort | Sponsor Opportunity |
|---------|-------------|--------|---------------------|
| **RDMA/InfiniBand Support** | High-speed networking | 3-4 weeks | $8,000 |
| **NVMe-oF Integration** | Remote NVMe storage | 2-3 weeks | $5,000 |
| **Triton/TVM Kernels** | ML compiler integration | 3-4 weeks | $6,000 |
| **Dynamic Precision Switching** | FP32/FP16/BF16/FP8 | 2 weeks | $4,000 |

### Future (Phase 6+)

| Feature | Description | Effort | Sponsor Opportunity |
|---------|-------------|--------|---------------------|
| **Custom Kubernetes Operator** | CRD-based deployment | 4-6 weeks | $15,000 |
| **MLOps Dashboard** | Web UI for monitoring | 4-6 weeks | $12,000 |
| **Hugging Face Integration** | Dataset/model hub | 2-3 weeks | $5,000 |
| **AWS/GCP/Azure Adapters** | Cloud provider support | 4-6 weeks | $10,000 |

---

## Roadmap Timeline

```
2025 Q1 (Jan-Mar)
├─ Phase 4: CUDA Integration
│  ├─ Native CUDA kernel loading
│  ├─ NCCL collective operations
│  └─ Multi-GPU memory management
│
├─ Phase 4: Production Testing
│  ├─ Kubernetes deployment validation
│  ├─ Load testing (1000+ concurrent jobs)
│  └─ Fault injection testing

2025 Q2 (Apr-Jun)
├─ Phase 5: HPC Features
│  ├─ RDMA/InfiniBand support
│  ├─ NVMe-oF storage integration
│  └─ Advanced NUMA optimizations
│
├─ Phase 5: Ecosystem
│  ├─ Triton/TVM kernel support
│  ├─ PyTorch native integration
│  └─ JAX backend support

2025 Q3 (Jul-Sep)
├─ Phase 6: Enterprise Features
│  ├─ Kubernetes Operator
│  ├─ Web Dashboard
│  └─ Multi-tenancy support
│
└─ v1.0 Stable Release
```

---

## Hardware Requirements

### Minimum (Development)
- **CPU**: 4 cores
- **RAM**: 8 GB
- **OS**: Linux (kernel 5.1+)
- **GPU**: None (CPU-only mode)

### Recommended (Testing)
- **CPU**: 8+ cores
- **RAM**: 32 GB
- **OS**: Ubuntu 22.04+ or RHEL 8+
- **GPU**: 1x NVIDIA GPU (Ampere+)

### Production (Full Features)
- **CPU**: 32+ cores, NUMA-capable
- **RAM**: 128+ GB
- **OS**: Enterprise Linux
- **GPU**: 4-8x NVIDIA H100/A100
- **Network**: 100Gbps+ InfiniBand (for RDMA)
- **Storage**: NVMe SSDs

---

## How to Contribute

We welcome contributions! Here's how you can help:

### Code Contributions

1. **Pick an issue** from our GitHub Issues
2. **Fork** the repository
3. **Implement** your feature/fix
4. **Write tests** (we require >80% coverage)
5. **Submit a PR**

### Priority Areas

| Area | Difficulty | Impact |
|------|------------|--------|
| Unit tests for uncovered code | Easy | High |
| Documentation improvements | Easy | Medium |
| Kubernetes testing | Medium | High |
| GPU feature implementation | Hard | Very High |
| RDMA/InfiniBand support | Hard | Very High |

### Testing Help Needed

We especially need help testing on:
- Multi-GPU systems (NVLink/NVSwitch)
- InfiniBand clusters
- Large Kubernetes deployments (100+ nodes)
- ARM64 platforms (AWS Graviton)

---

## Sponsorship Opportunities

### Why Sponsor Zenith?

- **Open Source**: Apache 2.0 licensed, forever free
- **High Impact**: Addresses real ML infrastructure challenges
- **Active Development**: Regular updates and maintenance
- **Community**: Growing community of ML engineers

### Sponsorship Tiers

| Tier | Amount | Benefits |
|------|--------|----------|
| **Bronze** | $500/month | Logo on README, priority support |
| **Silver** | $2,000/month | Bronze + quarterly roadmap input |
| **Gold** | $5,000/month | Silver + dedicated Slack channel |
| **Platinum** | $10,000/month | Gold + feature prioritization |
| **Enterprise** | Custom | Full partnership, custom features |

### One-Time Feature Sponsorship

| Feature | Sponsorship Amount | Estimated Delivery |
|---------|--------------------|--------------------|
| CUDA Kernel Integration | $5,000 | 3 weeks |
| NCCL Collectives | $5,000 | 3 weeks |
| RDMA Support | $8,000 | 4 weeks |
| Kubernetes Operator | $15,000 | 6 weeks |
| Custom Feature | Contact us | TBD |

### Current Sponsors

*Be the first sponsor! Your logo here.*

---

## Contact

### Author

**Wahyu Ardiansyah**
- GitHub: [@vibeswithkk](https://github.com/vibeswithkk)
- LinkedIn: [Connect](https://linkedin.com/in/)
- Email: For sponsorship inquiries

### Project Links

- **Repository**: [github.com/vibeswithkk/Zenith-dataplane](https://github.com/vibeswithkk/Zenith-dataplane)
- **Issues**: [Report bugs or request features](https://github.com/vibeswithkk/Zenith-dataplane/issues)
- **Discussions**: [Community discussions](https://github.com/vibeswithkk/Zenith-dataplane/discussions)

---

## License

Zenith is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

---

<div align="center">

**Built with passion for the ML community**

*If this project helps you, please consider giving it a star*

</div>
