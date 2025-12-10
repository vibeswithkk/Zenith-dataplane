# Zenith Tutorial: Getting Started

**Author:** Wahyu Ardiansyah  
**Version:** 0.2.2  
**Last Updated:** 2024-12-10

Welcome to the official tutorial for **Zenith DataPlane**. This guide will take you from zero to running a high-performance data pipeline.

---

## What is Zenith?

Zenith is a high-performance data loading engine built in Rust for AI/ML workloads. It provides:

- **4.2x faster** data loading compared to streaming iterators
- **Sub-millisecond latency** (p99: 0.074ms)
- **Zero-copy** data transfers via Apache Arrow
- **WASM plugin** support for custom preprocessing

---

## Prerequisites

| Requirement     | Version   | Purpose                 |
|-----------------|-----------|-------------------------|
| Rust            | 1.75+     | Core engine compilation |
| Python          | 3.10+     | SDK usage               |
| Cargo           | Latest    | Rust package manager    |
| (Optional) CUDA | 11.8+     | GPU acceleration        |

---

## Step 1: Installation

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/vibeswithkk/Zenith-dataplane.git
cd Zenith-dataplane

# Build all Rust components
cargo build --release

# Verify build
cargo test --workspace
```

### Python SDK Setup

```bash
# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install SDK in development mode
cd sdk-python
pip install -e .

# Verify installation
python3 -c "import zenith; print(zenith.info())"
```

---

## Step 2: Your First Data Load

### Using Python SDK

```python
import zenith

# Simple data loading
data = zenith.load("path/to/data.parquet")
print(f"Loaded {len(data)} records")

# Check system info
print(zenith.info())
```

### With PyTorch

```python
import zenith.torch as zt
import torch

# Create Zenith DataLoader
loader = zt.DataLoader(
    source="path/to/training_data",
    batch_size=64,
    num_workers=4,
    pin_memory=True
)

# Training loop
model = YourModel()
optimizer = torch.optim.Adam(model.parameters())

for epoch in range(10):
    for batch in loader:
        outputs = model(batch)
        loss = criterion(outputs, targets)
        loss.backward()
        optimizer.step()
        optimizer.zero_grad()
```

---

## Step 3: Understanding the Architecture

```
┌─────────────────────────────────────────────────┐
│                 Python Application              │
│         (PyTorch, TensorFlow, etc.)             │
└─────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────┐
│               Python SDK Layer                  │
│    zenith.load() / zenith.DataLoader            │
└─────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────┐
│               Rust Core Engine                  │
│  ┌───────────┐  ┌────────────┐  ┌────────────┐  │
│  │ DataLoader│  │ Arrow IPC  │  │ Prefetch   │  │
│  │ (Format)  │  │ (Zero-copy)│  │ Pipeline   │  │
│  └───────────┘  └────────────┘  └────────────┘  │
└─────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────┐
│                   Storage                       │
│     Parquet / Arrow IPC / CSV / S3              │
└─────────────────────────────────────────────────┘
```

### Key Concepts

| Concept          | Description                                     |
|------------------|-------------------------------------------------|
| **Ring Buffer**  | Lock-free circular buffer for event streaming   |
| **Zero-Copy**    | Data accessed in-place without copying          |
| **Arrow IPC**    | Efficient binary columnar format                |
| **WASM Plugins** | Sandboxed preprocessing in WebAssembly          |

---

## Step 4: Creating a WASM Plugin

Plugins allow custom data preprocessing in a sandboxed environment.

### Create Plugin Project

```bash
cargo new --lib my_plugin
cd my_plugin
```

### Configure Cargo.toml

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "z"
lto = true
```

### Write Plugin Code

```rust
// src/lib.rs
#![no_std]
extern crate alloc;

/// Called for each event - return true to allow, false to filter
#[no_mangle]
pub extern "C" fn on_event(source_id: u32, seq_no: u64) -> bool {
    // Example: Only allow events from source 1
    source_id == 1
}

/// Plugin metadata
#[no_mangle]
pub extern "C" fn plugin_info() -> *const u8 {
    static INFO: &[u8] = b"my-plugin v0.1.0\0";
    INFO.as_ptr()
}
```

### Build and Use

```bash
# Build to WASM
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release

# Output: target/wasm32-wasip1/release/my_plugin.wasm
```

```python
# Use in Python
import zenith

engine = zenith.Engine()
engine.load_plugin("path/to/my_plugin.wasm")
```

---

## Step 5: Running Benchmarks

Zenith includes a comprehensive benchmark suite.

### Generate Test Data

```bash
cd bench
python3 generate_datasets.py --size tiny
```

### Run Benchmarks

```bash
# Zenith benchmark
python3 zenith/zenith_benchmark.py

# Compare with PyTorch baseline
python3 baselines/pytorch_baseline.py
```

### Expected Results

| Metric      | Baseline | Zenith  | Improvement |
|-------------|----------|---------|-------------|
| Throughput  | 320K/s   | 1.35M/s | **4.2x**    |
| Latency p50 | 0.050ms  | 0.044ms | 1.1x        |
| Latency p99 | 0.134ms  | 0.074ms | 1.8x        |

---

## Step 6: Using the Job Scheduler

For distributed workloads, Zenith includes a job scheduler.

### Submit a Job

```python
import zenith

# Simple job submission
job_id = zenith.submit(
    command="python train.py",
    gpu_count=2,
    cpu_cores=4
)
print(f"Submitted job: {job_id}")

# Using decorator
@zenith.job(gpu_count=1, priority=100)
def train_model():
    # Training code here
    pass

train_model()  # Automatically scheduled
```

### Check Job Status

```bash
# REST API
curl http://localhost:8080/api/v1/jobs/{job_id}

# Cluster status
curl http://localhost:8080/api/v1/cluster/status
```

---

## Step 7: Supported Data Formats

| Format        | Extension | Zero-Copy | Notes |
|---------------|-----------|-----------|-------|
| Apache Parquet| `.parquet`| Partial   | Columnar, compressed |
| Arrow IPC     | `.arrow`, `.ipc` | Full | Best performance |
| CSV           | `.csv`           | No   | Human readable   |
| S3 Object     | `s3://...`       | Streaming | Cloud storage |

### Loading Different Formats

```python
import zenith

# Parquet
data = zenith.load("data.parquet")

# Arrow IPC
data = zenith.load("data.arrow")

# CSV
data = zenith.load("data.csv")

# S3 (future)
data = zenith.load("s3://bucket/key.parquet")
```

---

## Troubleshooting

### Common Issues

| Issue                               | Solution                                    |
|-------------------------------------|---------------------------------------------|
| `Zenith core library not found`     | Run `cargo build --release` first           |        
| `No module named zenith`            | Activate venv, `pip install -e sdk-python`  |
| `io_uring not supported`            | Use Linux 5.1+, or Zenith will fallback     |
| `WASM plugin failed to load`        | Check WASM target: `wasm32-wasip1`          |

### Getting Help

- **Issues**: [GitHub Issues](https://github.com/vibeswithkk/Zenith-dataplane/issues)
- **Documentation**: See `docs/` folder
- **Architecture**: `docs/ARCHITECTURE.md`

---

## Next Steps

1. **Read the Plugin Guide**: `docs/PLUGIN_GUIDE.md`
2. **Explore Architecture**: `docs/ARCHITECTURE.md`
3. **Run Benchmarks**: `bench/README.md`
4. **Check QA Report**: `docs/QA_REPORT.md`

---

## Summary

You've learned how to:

- [x] Install Zenith from source
- [x] Load data with Python SDK
- [x] Integrate with PyTorch
- [x] Create WASM plugins
- [x] Run benchmarks
- [x] Submit jobs to scheduler

**Zenith provides the critical data loading infrastructure so your GPUs never starve for data!**

---

*Tutorial by Wahyu Ardiansyah*
