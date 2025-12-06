# Zenith AI: High-Performance Data Infrastructure for Machine Learning

[![PyPI](https://img.shields.io/pypi/v/zenith-ai?color=blue&label=PyPI)](https://pypi.org/project/zenith-ai/)
[![Downloads](https://img.shields.io/pypi/dm/zenith-ai?color=green&label=Downloads)](https://pypi.org/project/zenith-ai/)
[![Python](https://img.shields.io/pypi/pyversions/zenith-ai?label=Python)](https://pypi.org/project/zenith-ai/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Zenith CI](https://github.com/vibeswithkk/Zenith-dataplane/actions/workflows/ci.yml/badge.svg)](https://github.com/vibeswithkk/Zenith-dataplane/actions)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)

> **"Stop Starving Your GPUs. Feed Them with Zenith."**

**Zenith AI** is a high-performance data loading and preprocessing library designed to accelerate AI/ML training pipelines. Built with Rust for speed and Python for accessibility, Zenith eliminates the data bottleneck that causes GPU idle time during model training.

## Why Zenith?

| Problem | Zenith Solution |
|---------|-----------------|
| Python DataLoaders are slow | Rust-powered core with **< 100µs latency** |
| GPUs wait for data (idle time) | **Zero-copy architecture** keeps GPUs fed |
| Preprocessing blocks training | **WASM plugins** run transforms in parallel |
| Complex setup requirements | Simple `pip install zenith-ai` |

## Key Features

- **Blazing Fast**: Rust core delivers **6,000,000+ events/second** throughput
- **Zero-Copy Memory**: Apache Arrow integration eliminates serialization overhead
- **Framework Agnostic**: Works with PyTorch, TensorFlow, JAX, and more
- **Extensible Preprocessing**: Custom WASM plugins for image resize, tokenization, augmentation
- **Simple Integration**: Drop-in replacement for standard DataLoaders

## Installation

```bash
pip install zenith-ai
```

*For development/contribution, see [Building from Source](#building-from-source).*

## Quick Start

### Basic Usage (Standalone)

```python
import zenith

# Initialize the high-performance engine
engine = zenith.Engine()

# Load your data at blazing speed
data = engine.load("path/to/dataset")

# Apply preprocessing via WASM plugin
engine.load_plugin("image_resize.wasm")
processed = engine.process(data)
```

### PyTorch Integration

```python
import zenith.torch as zt
from torch.utils.data import DataLoader

# Zenith-accelerated dataset
dataset = zt.ZenithDataset(
    source="s3://your-bucket/training-data",
    preprocessing_plugin="tokenizer.wasm"
)

# Use as standard PyTorch DataLoader
loader = DataLoader(dataset, batch_size=64, num_workers=4)

for batch in loader:
    # Your training loop - GPU never waits!
    model.train_step(batch)
```

### TensorFlow Integration

```python
import zenith.tensorflow as ztf

# Create a tf.data compatible dataset
dataset = ztf.ZenithDataset(
    source="/data/imagenet",
    preprocessing_plugin="augmentation.wasm"
)

# Standard TensorFlow pipeline
dataset = dataset.batch(32).prefetch(tf.data.AUTOTUNE)
model.fit(dataset, epochs=10)
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    YOUR ML APPLICATION                      │
│              (PyTorch / TensorFlow / JAX)                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ZENITH ADAPTERS                          │
│         zenith.torch  │  zenith.tensorflow  │  zenith.jax   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ZENITH CORE ENGINE                       │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐   │
│  │ Data Loader │  │ Ring Buffer  │  │ WASM Preprocessor │   │
│  │   (Rust)    │  │ (Lock-Free)  │  │    (Wasmtime)     │   │
│  └─────────────┘  └──────────────┘  └───────────────────┘   │
│                    Apache Arrow (Zero-Copy)                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    DATA SOURCES                             │
│     Local Files  │  S3/GCS  │  Kafka  │  Database           │
└─────────────────────────────────────────────────────────────┘
```

## Performance Benchmarks

| Scenario                         | Standard PyTorch | Zenith    | Speedup  |
|----------------------------------|------------------|-----------|----------|
| ImageNet Loading (1TB)           | 45 min           | 8 min     | **5.6x** |
| Text Tokenization (10M docs)     | 12 min           | 2 min     | **6x**   |
| Real-time Inference (events/sec) | 50,000           | 6,000,000 | **120x** |

*Benchmarks on AMD EPYC 7763 + NVMe SSD. Your results may vary.*

## Components

| Component | Description | Status |
|-----------|-------------|--------|
| **Core Engine** | Rust-based high-performance data loading | ✅ Stable |
| **Python SDK** | PyArrow-compatible Python bindings | ✅ Stable |
| **PyTorch Adapter** | Native PyTorch DataLoader integration | 🚧 Beta |
| **TensorFlow Adapter** | tf.data compatible interface | 🚧 Beta |
| **WASM Plugins** | Custom preprocessing (image, text, audio) | ✅ Stable |
| **CLI Tools** | Benchmarking and debugging utilities | ✅ Stable |

## Building from Source

### Prerequisites
- Rust 1.75+
- Python 3.10+
- Maturin (for Python packaging)

### Steps

```bash
# Clone repository
git clone https://github.com/vibeswithkk/Zenith-dataplane.git
cd Zenith-dataplane

# Build Rust core
cargo build --release

# Build Python package (development mode)
cd sdk-python
pip install maturin
maturin develop

# Build WASM plugins
cd ../plugins/simple_filter
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
```

## Use Cases

### 1. Large Language Model (LLM) Training
Tokenize and stream terabytes of text data without Python bottlenecks.

### 2. Computer Vision at Scale
Decode, resize, and augment millions of images in real-time.

### 3. Real-time Inference Systems
Process sensor data, audio streams, or video frames with microsecond latency.

### 4. Multi-modal Training
Handle mixed data types (text, image, audio) in a unified, fast pipeline.

## Documentation

- [Architecture Overview](docs/ARCHITECTURE.md)
- [API Reference](docs/API_SPEC.md)
- [Plugin Development Guide](docs/PLUGIN_GUIDE.md)
- [Benchmark Report](docs/BENCHMARK_REPORT.md)
- [PyTorch Integration Guide](docs/PYTORCH_GUIDE.md)

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Roadmap

- [x] Core Rust Engine
- [x] Python SDK (ctypes)
- [x] WASM Plugin System
- [ ] PyTorch Adapter (zenith.torch)
- [ ] TensorFlow Adapter (zenith.tensorflow)
- [ ] JAX Adapter (zenith.jax)
- [ ] S3/GCS Native Streaming
- [ ] GPU Direct Storage Integration

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <b>Built for the AI Era. Powered by Rust. Loved by Data Scientists.</b>
</p>
