<div align="center">
  <img src="docs/assets/logo.png" alt="Zenith Logo" width="200" height="auto" />
  <h1>Zenith DataPlane</h1>
</div>



<div align="center">

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python-3.10%2B-blue.svg)](https://www.python.org/)
[![Build](https://img.shields.io/github/actions/workflow/status/vibeswithkk/Zenith-dataplane/ci.yml?branch=main)](https://github.com/vibeswithkk/Zenith-dataplane/actions)
[![Tests](https://img.shields.io/badge/tests-88%2B%20passing-brightgreen.svg)](docs/QA_REPORT.md)

> **High-Performance Data Infrastructure for the AI Era**
>
> *"Stop Starving Your GPUs. Feed Them with Zenith."*
</div>

---

## Vision

Zenith is a comprehensive data infrastructure ecosystem designed to accelerate AI/ML training and inference at scale. It provides:

1. **Zenith Core Engine** - High-performance data loading with Arrow IPC, zero-copy transfers, and WASM plugins
2. **Zenith CPU Runtime** - Ultra-low-latency CPU runtime with NUMA awareness, io_uring, and lock-free data structures
3. **Zenith GPU Runtime** - GPU-aware runtime with automatic kernel selection and topology-aware placement
4. **Zenith Job Scheduler** - Lightweight scheduler with gang scheduling, topology awareness, and preemption support

**Zenith doesn't replace PyTorch/TensorFlow** — it provides the data loading layer that keeps your GPUs fed and busy.

---

## Philosophy & Origin

Zenith was born from a desire to contribute meaningfully to the rapid evolution of Artificial Intelligence. In a world where compute is precious, this project represents a personal commitment to the technological community: to build infrastructure that honors the efficiency of engineering and the ambition of creators.

It stands as a bridge between high-performance systems and the democratized future of AI—ensuring that as models grow larger, the tools that feed them remain elegant, open, and incredibly fast.

---

## Performance

Benchmarked on Linux x86_64 with NVMe SSD:

| Metric               | Baseline      | Zenith        | Improvement     |
|----------------------|---------------|---------------|-----------------|
| Data Loading         | 320K samples/s| 1.35M samples/s| **4.2x faster** |
| Latency (P99)        | 0.134 ms      | 0.074 ms      | **1.8x lower**  |
| Latency (P50)        | 0.050 ms      | 0.044 ms      | **1.1x lower**  |

*See [Benchmark Report](bench/reports/BENCHMARK_REPORT.md) for full methodology and results.*

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ZENITH ECOSYSTEM                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │  Core Engine    │  │  CPU Runtime    │  │   GPU Runtime       │  │
│  │  ─────────────  │  │  ─────────────  │  │   ─────────────     │  │
│  │  • Arrow IPC    │  │  • NUMA Aware   │  │   • CUDA/TensorRT   │  │
│  │  • Zero-copy    │  │  • io_uring     │  │   • Multi-GPU       │  │
│  │  • WASM Plugins │  │  • Lock-free    │  │   • Memory Mgmt     │  │
│  │  • DataLoader   │  │  • Ring Buffers │  │   • NVML Monitor    │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    Python SDK                                   ││
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────┐    ││
│  │  │ zenith.torch  │  │ zenith.tf     │  │ zenith.load()     │    ││
│  │  └───────────────┘  └───────────────┘  └───────────────────┘    ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    Job Scheduler                                ││
│  │  • Gang Scheduling  • Priority Queues  • REST/gRPC API          ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Quick Start

### Installation

```bash
# From source (recommended)
git clone https://github.com/vibeswithkk/Zenith-dataplane.git
cd Zenith-dataplane
cargo build --release

# Python SDK (development)
cd sdk-python
pip install -e .
```

### Python Usage

```python
import zenith

# Load data efficiently
data = zenith.load("path/to/data.parquet")
print(f"Loaded {len(data)} records")

# Use with PyTorch
import zenith.torch as zt

loader = zt.DataLoader(
    source="path/to/training_data",
    batch_size=64,
    num_workers=4,
    pin_memory=True
)

for batch in loader:
    outputs = model(batch)
    loss.backward()
    optimizer.step()
```

### Rust Usage

```rust
use zenith_runtime_cpu::dataloader::{DataLoader, DataLoaderConfig, DataSource};

let config = DataLoaderConfig::default()
    .with_batch_size(1024)
    .with_prefetch_batches(4);

let source = DataSource::from_path("data.parquet");
let loader = DataLoader::new(source, config)?;

for batch in loader {
    // Process batch
}
```

---

## Repository Structure

```
zenith-dataplane/
├── core/                   # Core engine (FFI, WASM host, validation)
├── zenith-runtime-cpu/     # CPU runtime (NUMA, io_uring, SIMD)
├── zenith-runtime-gpu/     # GPU runtime (CUDA, TensorRT, Multi-GPU)
├── zenith-scheduler/       # Job scheduler (gang scheduling, REST/gRPC)
├── sdk-python/             # Python SDK
├── bench/                  # Benchmarks
├── docs/                   # Documentation
└── examples/               # Example code
```

---

## Documentation

| Document                                           | Description                  |
|----------------------------------------------------|------------------------------|
| [Architecture](docs/ARCHITECTURE.md)               | System design and components |
| [Plugin Guide](docs/PLUGIN_GUIDE.md)               | WASM plugin development      |
| [Implementation](docs/IMPLEMENTATION.md)           | Technical implementation     |
| [Benchmark Report](bench/reports/BENCHMARK_REPORT.md) | Performance benchmarks    |
| [QA Report](docs/QA_REPORT.md)                     | Quality assurance metrics    |
| [Technical Proposal](docs/TECHNICAL_PROPOSAL.md)   | Academic-style proposal      |
| [Changelog](CHANGELOG.md)                          | Version history              |

---

## Development

### Prerequisites

- Rust 1.75+
- Python 3.10+
- (Optional) CUDA Toolkit 11.8+ for GPU features

### Building

```bash
# Build all crates
cargo build --release

# Run tests
cargo test --workspace

# Run specific package tests
cargo test -p zenith-core
cargo test -p zenith-runtime-cpu
cargo test -p zenith-scheduler
```

### Test Status

```
zenith-core:        5+ tests passing
zenith-runtime-cpu: 52+ tests passing
zenith-scheduler:   31+ tests passing
─────────────────────────────────
Total:              88+ tests passing
```

---

## Technical References

This project is built upon research and industry best practices:

1. **Apache Arrow** - Zero-Copy Data ([arrow.apache.org](https://arrow.apache.org/))
2. **io_uring** - Linux Async I/O ([man7.org](https://man7.org/linux/man-pages/man7/io_uring.7.html))
3. **WebAssembly** - Plugin Sandboxing ([webassembly.org](https://webassembly.org/))
4. **NVIDIA NCCL** - Collective Communications ([docs](https://docs.nvidia.com/deeplearning/nccl/))
5. **Slurm** - Gang Scheduling Concepts ([SchedMD](https://slurm.schedmd.com/))

---

## Author

**Wahyu Ardiansyah** (Indonesia)

- GitHub: [@vibeswithkk](https://github.com/vibeswithkk)

---

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

```
Copyright 2025 Wahyu Ardiansyah
```

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## Support

If you find Zenith useful, please consider giving it a star on GitHub!

For issues or questions, please [open an issue](https://github.com/vibeswithkk/Zenith-dataplane/issues).
