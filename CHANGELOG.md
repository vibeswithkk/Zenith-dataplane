# Changelog

All notable changes to Zenith AI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- JAX adapter integration
- S3/GCS native streaming
- GPU Direct Storage support
- Distributed data loading

---

## [0.1.0] - 2025-12-07

### 🎉 Initial Release

The first public release of Zenith AI - High-Performance Data Infrastructure for Machine Learning.

### Added

#### Core Engine
- **Native Rust Core**: High-performance data processing engine written in Rust
- **PyO3 Bindings**: Native Python extension for maximum performance
- **Lock-free Ring Buffer**: SPSC (Single Producer Single Consumer) buffer for low-latency data streaming
- **Zero-copy Architecture**: Apache Arrow integration for efficient memory management

#### Python SDK
- **zenith.Engine**: Core engine class with plugin support
- **zenith.DataLoader**: Framework-agnostic high-performance data loader
- **Automatic fallback**: Uses native Rust when available, Python fallback otherwise

#### Framework Adapters
- **zenith.torch**: PyTorch integration
  - `ZenithDataset`: Iterable dataset for streaming large datasets
  - `ZenithMapDataset`: Map-style dataset for random access
  - `DataLoader`: Drop-in replacement for torch.utils.data.DataLoader
- **zenith.tensorflow**: TensorFlow integration
  - `ZenithDataset`: tf.data compatible dataset

#### WASM Plugin System
- **Plugin Runtime**: Secure WebAssembly execution via Wasmtime
- **Image Operations Plugin**: Resize, normalize, augmentation
- **Text Operations Plugin**: Tokenization, text cleaning, padding

#### Tools & Utilities
- **benchmark.py**: Performance testing tool
- **build_plugin.sh**: WASM plugin builder script
- **inspector.py**: Engine monitoring utility

#### Documentation
- Architecture overview
- PyTorch integration guide
- TensorFlow integration guide
- Plugin development guide
- API specification
- Publishing guide

### Performance
- **Throughput**: 6,000,000+ events/second validated
- **Latency**: < 100µs data loading latency
- **Memory**: Zero-copy data transfer via Apache Arrow

### Platforms
- Linux x86_64 (primary)
- macOS (planned)
- Windows (planned)

### Python Versions
- Python 3.10+
- Python 3.11+
- Python 3.12+

---

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| 0.1.0 | 2025-12-07 | Initial release with PyTorch/TensorFlow support |

---

[Unreleased]: https://github.com/vibeswithkk/Zenith-dataplane/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/vibeswithkk/Zenith-dataplane/releases/tag/v0.1.0
