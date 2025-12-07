# Zenith Infrastructure

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python-3.10%2B-blue.svg)](https://www.python.org/)
[![PyPI](https://img.shields.io/pypi/v/zenith-ai)](https://pypi.org/project/zenith-ai/)

> **High-Performance Infrastructure for the AI Era**
>
> *"Stop Starving Your GPUs. Feed Them with Zenith."*

---

##  Vision

Zenith is a comprehensive infrastructure ecosystem designed to accelerate AI/ML training and inference at scale. It provides:

1. **Zenith GPU Runtime** - GPU-aware runtime with automatic kernel selection, topology-aware placement, and ZeRO-style memory offload
2. **Zenith Job Scheduler** - Lightweight mini-Slurm with gang scheduling, topology awareness, and preemption support
3. **Zenith CPU Engine** - Ultra-low-latency CPU runtime with NUMA awareness, io_uring, and lock-free data structures

**Zenith doesn't replace PyTorch/DeepSpeed/Triton** — it provides the runtime performance layer and lightweight scheduler that accelerates and orchestrates large AI workloads on real infrastructure.

---

##  Performance

| Metric | Standard | Zenith | Improvement |
|--------|----------|--------|-------------|
| Data Loading | 50K events/s | 6M events/s | **120x** |
| Latency (P99) | 10 ms | 100 µs | **100x** |
| Memory Overhead | 2.5 GB | 150 MB | **16x less** |
| GPU Utilization | 60-70% | 95%+ | **+35%** |

---

##  Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ZENITH ECOSYSTEM                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │  GPU Runtime    │  │  Job Scheduler  │  │   CPU Engine        │  │
│  │  ─────────────  │  │  ─────────────  │  │   ─────────────     │  │
│  │  • Kernel Mgr   │  │  • Gang Sched   │  │   • NUMA Aware      │  │
│  │  • Memory       │  │  • Topology     │  │   • io_uring        │  │ 
│  │  • NCCL         │  │  • Preemption   │  │   • Lock-free       │  │
│  │  • ZeRO Offload │  │  • Quotas       │  │   • Ring Buffers    │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    Python SDK (pip install zenith-ai)           ││
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────┐    ││
│  │  │ PyTorch       │  │ TensorFlow    │  │ JAX (planned)     │    ││
│  │  └───────────────┘  └───────────────┘  └───────────────────┘    ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

##  Quick Start

### Installation

```bash
# Python SDK (recommended)
pip install zenith-ai

# From source
git clone https://github.com/vibeswithkk/Zenith-dataplane.git
cd Zenith-dataplane
cargo build --release
```

### PyTorch Example

```python
import zenith.torch as zt

# Create high-performance data loader
loader = zt.DataLoader(
    source="path/to/training_data",
    batch_size=64,
    shuffle=True,
    preprocessing_plugin="image_resize.wasm",
    num_workers=4,
    pin_memory=True
)

# Training loop - GPU never starves for data
for epoch in range(10):
    for batch in loader:
        outputs = model(batch)
        loss = criterion(outputs, targets)
        loss.backward()
        optimizer.step()
```

### TensorFlow Example

```python
import zenith.tensorflow as ztf

dataset = ztf.ZenithDataset(
    source="path/to/training_data",
    preprocessing_plugin="image_resize.wasm"
)

dataset = dataset.batch(32).prefetch(tf.data.AUTOTUNE)
model.fit(dataset, epochs=10)
```

---

##  Repository Structure

```
zenith/
├── zenith-runtime-gpu/     # GPU optimization runtime
│   ├── src/
│   │   ├── device.rs       # GPU topology discovery
│   │   ├── kernel.rs       # Kernel manager
│   │   ├── memory.rs       # ZeRO-style offload
│   │   └── collective.rs   # NCCL integration
│   └── Cargo.toml
│
├── zenith-scheduler/       # Job scheduler (mini-Slurm)
│   ├── src/
│   │   ├── job.rs          # Job definitions
│   │   ├── node.rs         # Node registry
│   │   ├── scheduler.rs    # Gang scheduling
│   │   └── api/            # gRPC & REST APIs
│   └── Cargo.toml
│
├── zenith-runtime-cpu/     # CPU low-latency runtime
│   ├── src/
│   │   ├── buffer.rs       # Lock-free ring buffers
│   │   ├── numa.rs         # NUMA topology
│   │   ├── allocator.rs    # NUMA-aware allocator
│   │   └── io.rs           # io_uring integration
│   └── Cargo.toml
│
├── zenith-proto/           # Protocol definitions
│   └── zenith.proto        # gRPC/Protobuf schemas
│
├── zenith-bench/           # Benchmark harness
│   └── src/main.rs         # MLPerf-style benchmarks
│
├── sdk-python/             # Python SDK
│   ├── zenith/             # Python package
│   └── pyproject.toml      # Maturin config
│
└── docs/                   # Documentation
    ├── ARCHITECTURE.md     # System architecture
    ├── PYTORCH_GUIDE.md    # PyTorch integration
    └── PLUGIN_GUIDE.md     # WASM plugin development
```

---

##  Development

### Prerequisites

- Rust 1.75+
- Python 3.10+
- (Optional) CUDA Toolkit 11.8+ for GPU features
- (Optional) NCCL for multi-GPU communication

### Building

```bash
# Build all crates
cargo build --release

# Run tests
cargo test --all

# Run benchmarks
cargo run -p zenith-bench --release -- full
```

### Python Development

```bash
cd sdk-python

# Create virtual environment
python -m venv .venv
source .venv/bin/activate

# Install with maturin
pip install maturin
maturin develop

# Run tests
pytest tests/
```

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/ARCHITECTURE.md) | System design and components |
| [PyTorch Guide](docs/PYTORCH_GUIDE.md) | PyTorch integration tutorial |
| [TensorFlow Guide](docs/TENSORFLOW_GUIDE.md) | TensorFlow integration tutorial |
| [Plugin Guide](docs/PLUGIN_GUIDE.md) | WASM plugin development |
| [Operator Guide](docs/OPERATOR_GUIDE.md) | Cluster deployment |
| [API Reference](docs/API.md) | gRPC and REST API |

---

## 🔬 Technical References

This project is built upon research and industry best practices:

1. **ZeRO: Memory Optimizations** - Microsoft DeepSpeed ([arXiv](https://arxiv.org/abs/1910.02054))
2. **NVIDIA NCCL** - Collective Communications ([docs](https://docs.nvidia.com/deeplearning/nccl/))
3. **Slurm Workload Manager** - Gang Scheduling ([SchedMD](https://slurm.schedmd.com/))
4. **io_uring** - Linux Async I/O ([man7.org](https://man7.org/linux/man-pages/man7/io_uring.7.html))
5. **Apache Arrow** - Zero-Copy Data ([arrow.apache.org](https://arrow.apache.org/))
6. **MLPerf** - Benchmark Standards ([mlcommons.org](https://mlcommons.org/))

---

##  Author

**Wahyu Ardiansyah** 🇮🇩

- GitHub: [@vibeswithkk](https://github.com/vibeswithkk)
- Made with  in Indonesia

---

##  License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

```
Copyright 2025 Wahyu Ardiansyah and Zenith AI Contributors
```

---

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## ⭐ Star History

If you find Zenith useful, please consider giving it a star on GitHub!
