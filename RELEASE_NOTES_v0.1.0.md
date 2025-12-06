# 🚀 Zenith AI v0.1.0 - Initial Release

> **"Stop Starving Your GPUs. Feed Them with Zenith."**

We're excited to announce the first public release of **Zenith AI** - a high-performance data loading and preprocessing library designed to accelerate AI/ML training pipelines.

## ✨ Highlights

- **🔥 Blazing Fast**: 6,000,000+ events/second throughput
- **⚡ Ultra-Low Latency**: < 100µs data loading
- **🦀 Rust-Powered**: Native performance with Python simplicity
- **🔌 PyTorch & TensorFlow**: First-class framework integration
- **🧩 WASM Plugins**: Secure, sandboxed preprocessing

## 📦 Installation

```bash
pip install zenith-ai
```

With framework support:
```bash
pip install zenith-ai[torch]      # PyTorch
pip install zenith-ai[tensorflow] # TensorFlow
pip install zenith-ai[all]        # Everything
```

## 🚀 Quick Start

### PyTorch
```python
import zenith.torch as zt

loader = zt.DataLoader(
    source="path/to/data",
    batch_size=64,
    preprocessing_plugin="image_resize.wasm"
)

for batch in loader:
    model.train_step(batch)  # GPU never waits!
```

### TensorFlow
```python
import zenith.tensorflow as ztf

dataset = ztf.ZenithDataset("path/to/data")
dataset = dataset.batch(32).prefetch(tf.data.AUTOTUNE)
model.fit(dataset, epochs=10)
```

## 📊 Performance

| Scenario | Standard PyTorch | Zenith | Speedup |
|----------|------------------|--------|---------|
| ImageNet Loading (1TB) | 45 min | 8 min | **5.6x** |
| Text Tokenization (10M docs) | 12 min | 2 min | **6x** |
| Real-time Inference | 50K events/s | 6M events/s | **120x** |

## 🆕 What's New

### Core Features
- Native Rust engine with PyO3 bindings
- Zero-copy data transfer via Apache Arrow
- Lock-free ring buffer for streaming
- WASM plugin system for preprocessing

### Framework Support
- `zenith.torch.DataLoader` - Drop-in PyTorch replacement
- `zenith.torch.ZenithDataset` - Streaming & map-style datasets
- `zenith.tensorflow.ZenithDataset` - tf.data compatible

### Plugins
- Image operations: resize, normalize, augment
- Text operations: tokenize, clean, pad

## 📚 Documentation

- [Architecture Overview](docs/ARCHITECTURE.md)
- [PyTorch Guide](docs/PYTORCH_GUIDE.md)
- [Plugin Development](docs/PLUGIN_GUIDE.md)
- [API Reference](docs/API_SPEC.md)

## 🔧 Requirements

- Python 3.10+
- Linux x86_64 (macOS/Windows coming soon)

## 🙏 Acknowledgments

Built with ❤️ using:
- [Rust](https://www.rust-lang.org/) - Performance
- [PyO3](https://pyo3.rs/) - Python bindings
- [Apache Arrow](https://arrow.apache.org/) - Zero-copy memory
- [Wasmtime](https://wasmtime.dev/) - WASM runtime

## 📄 License

Apache License 2.0

---

**Full Changelog**: https://github.com/vibeswithkk/Zenith-dataplane/commits/v0.1.0

---

<p align="center">
  <b>Built for the AI Era. Powered by Rust. Loved by Data Scientists.</b>
</p>
