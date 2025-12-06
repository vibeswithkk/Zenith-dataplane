# Zenith AI Python SDK

High-performance data loading and preprocessing for machine learning.

## Installation

```bash
pip install zenith-ai
```

With framework support:
```bash
pip install zenith-ai[torch]      # PyTorch integration
pip install zenith-ai[tensorflow] # TensorFlow integration
pip install zenith-ai[all]        # All frameworks
```

## Quick Start

### Basic Usage

```python
import zenith

# Initialize the high-performance engine
engine = zenith.Engine()

# Load preprocessing plugin
engine.load_plugin("image_resize.wasm")

# Load and process data
data = engine.load("path/to/dataset.parquet")
```

### PyTorch Integration

```python
import zenith.torch as zt

loader = zt.DataLoader(
    source="data/train.parquet",
    batch_size=64,
    shuffle=True,
    preprocessing_plugin="normalize.wasm"
)

for batch in loader:
    model.train_step(batch)
```

### TensorFlow Integration

```python
import zenith.tensorflow as ztf

dataset = ztf.ZenithDataset("data/train")
dataset = dataset.batch(32).prefetch(tf.data.AUTOTUNE)

model.fit(dataset, epochs=10)
```

## Features

- **Ultra-fast data loading** (< 100µs latency)
- **Zero-copy memory** via Apache Arrow
- **WASM preprocessing** plugins
- **Framework agnostic** (PyTorch, TensorFlow, JAX)

## Development

```bash
# Clone repository
git clone https://github.com/vibeswithkk/Zenith-dataplane.git
cd Zenith-dataplane

# Build Rust core
cargo build --release

# Install in development mode
cd sdk-python
pip install maturin
maturin develop
```

## License

Apache License 2.0
