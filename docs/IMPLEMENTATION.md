# Zenith Implementation Documentation
**Author:** Wahyu Ardiansyah 
**Version:** 0.2.0 
**Date:** 2024-12-09 
---
## Table of Contents

1. [Project Overview](#project-overview)
2. [Architecture](#architecture)
3. [Implementation Details](#implementation-details)
4. [Testing & Quality Assurance](#testing--quality-assurance)
5. [Benchmark Results](#benchmark-results)
6. [API Reference](#api-reference)
7. [Deployment Guide](#deployment-guide)
---
## 1. Project Overview
### Vision

Zenith is a high-performance data ingestion and dataplane engine designed for ML/streaming workloads. It reduces I/O bottlenecks and improves end-to-end training throughput.
### Core Goals
- **2-5x faster data loading** than native PyTorch DataLoader
- **Sub-millisecond batch latencies** for GPU utilization optimization
- **Zero-copy memory sharing** via Apache Arrow integration
- **Drop-in replacement** for existing ML frameworks
### Key Features (v0.2.0)

| Feature | Status | Description |
|---------|--------|-------------|
| Zero-copy Arrow IPC | [OK] Complete | Direct memory access without serialization |
| Parquet/CSV/Arrow loading | [OK] Complete | Multi-format support |
| PyTorch adapter | [OK] Complete | `from zenith.torch import DataLoader` |
| Batch iteration | [OK] Complete | Efficient batch processing |
| Multi-worker prefetch | In Progress | Parallel data loading |
| WebDataset support | Planned | TAR shard streaming |
| S3 adapter | Planned | Cloud storage integration |
---
## 2. Architecture
### System Architecture

```

 Python SDK Layer 
 
 zenith.load DataLoader zenith.torch.DataLoader 
 

 FFI (ctypes)

 Rust Core Engine 
 
 zenith-core cpu-runtime dataloader 
 (FFI + Engine) (NUMA/SIMD) (Arrow/Parquet) 
 

```
### Component Breakdown

| Component | Language | Purpose |
|-----------|----------|---------|
| `core/` | Rust | FFI bindings, engine coordination |
| `zenith-runtime-cpu/` | Rust | CPU runtime, NUMA, SIMD, DataLoader |
| `zenith-scheduler/` | Rust | Job scheduling (SLURM alternative) |
| `sdk-python/` | Python | User-facing API |
| `bench/` | Python/Bash | Benchmark suite |
---
## 3. Implementation Details
### 3.1 Rust Core DataLoader
**File:** `zenith-runtime-cpu/src/dataloader.rs`

```rust
pub struct DataLoader {
 config: LoaderConfig,
 source: DataSource,
 schema: Option<Arc<Schema>>,
 cached_batches: RwLock<Option<Vec<RecordBatch>>>,
}

impl DataLoader {
 pub fn load(&self) -> Result<BatchIterator, DataLoaderError>;
 pub fn load_parquet(&self, path: &str) -> Result<...>;
 pub fn load_csv(&self, path: &str) -> Result<...>;
 pub fn load_arrow_ipc(&self, path: &str) -> Result<...>;
}
```
**Features:**
- Zero-copy Arrow IPC reading
- Automatic format detection
- Result caching for small datasets (<100MB)
- Batch iteration support
### 3.2 Python SDK
**File:** `sdk-python/zenith/__init__.py`

```python
import zenith
# Simple loading
data = zenith.load("train.parquet")
# DataLoader usage
loader = zenith.DataLoader("data/", batch_size=64)
# Job scheduling
@zenith.job(gpus=4)
def train(): ...
zenith.submit(train)
```
### 3.3 FFI Safety
**File:** `core/src/lib.rs`

All FFI functions wrapped with `catch_unwind` to prevent Rust panics from crashing Python:

```rust
#[no_mangle]
pub extern "C" fn zenith_init(config: *const c_char) -> i32 {
 let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
 // ... implementation
 }));
 
 match result {
 Ok(r) => r,
 Err(_) => ffi_error::FFI_PANIC,
 }
}
```
### 3.4 Input Validation
**File:** `core/src/validation.rs`

Comprehensive validation for security:

```rust
pub struct Validator {
 forbidden_patterns: HashSet<String>,
}

impl Validator {
 pub fn validate_job_name(&self, name: &str) -> ValidationResult<()>;
 pub fn validate_path(&self, path: &str) -> ValidationResult<()>;
 pub fn validate_command(&self, command: &str) -> ValidationResult<()>;
}
```
---
## 4. Testing & Quality Assurance
### 4.1 Test Coverage

| Module | Tests | Status |
|--------|-------|--------|
| zenith-runtime-cpu | 49 tests | [OK] All passing |
| zenith-core | 5 tests | [OK] All passing |
| zenith-scheduler | 10+ tests | [OK] All passing |
| **Total** | **54+ tests** | **100% pass rate** |
### 4.2 Running Tests

```bash
# Run all Rust tests
cargo test
# Run specific package
cargo test -p zenith-runtime-cpu
# Run with output
cargo test -- --nocapture
```
### 4.3 Quality Metrics

| Metric | Standard | Zenith Status |
|--------|----------|---------------|
| Compilation | No errors | [OK] Clean |
| Warnings | Minimal | [OK] 29 (docs only) |
| FFI Safety | catch_unwind | [OK] Implemented |
| Input Validation | Required | [OK] Implemented |
---
## 5. Benchmark Results
### 5.1 Summary

| Metric | Zenith | Improvement |
|--------|--------|-------------|
| Throughput | 1.35M samples/sec | **4.2x vs iterator** |
| Latency p50 | 0.044 ms | Sub-millisecond |
| Latency p99 | 0.074 ms | Ultra-low tail latency |
### 5.2 MVP Criteria

| Criteria | Target | Result | Status |
|----------|--------|--------|--------|
| ≥20% throughput improvement | Required | **322%** | [OK] PASS |
| Reproducible benchmarks | Required | Scripts available | [OK] PASS |
| Documentation | Required | Complete | [OK] PASS |
### 5.3 Benchmark Commands

```bash
# Generate datasets
python bench/generate_datasets.py --scale tiny
# Run Zenith benchmark
python bench/zenith/zenith_benchmark.py --duration 10 --batch-size 64
# Full suite
./bench/run_benchmarks.sh --all
```
---
## 6. API Reference
### Python API

```python
# Core functions
zenith.load(path: str) -> pyarrow.Table
zenith.info() -> None
# DataLoader
zenith.DataLoader(source, batch_size=32, shuffle=True)
# Job scheduling
@zenith.job(gpus=1, memory="8GB")
def func(): ...

zenith.submit(func)
zenith.status(job_id)
```
### Rust API

```rust
// DataLoader
pub fn DataLoader::new(source: DataSource, config: LoaderConfig) -> Self;
pub fn DataLoader::load(&self) -> Result<BatchIterator, DataLoaderError>;

// BatchIterator
impl Iterator for BatchIterator {
 type Item = RecordBatch;
 fn next(&mut self) -> Option<RecordBatch>;
}
```
---
## 7. Deployment Guide
### 7.1 Installation

```bash
# From source
git clone https://github.com/vibeswithkk/Zenith-dataplane.git
cd Zenith-dataplane
cargo build --release
pip install -e sdk-python/
```
### 7.2 Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ZENITH_CORE_LIB` | Path to libzenith_core.so | Auto-detect |
| `ZENITH_LOG_LEVEL` | Logging verbosity | `info` |
| `ZENITH_NUM_WORKERS` | Default worker count | `4` |
### 7.3 Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM python:3.12-slim
COPY --from=builder /app/target/release/libzenith_core.so /usr/lib/
COPY sdk-python/ /app/sdk-python/
RUN pip install -e /app/sdk-python/
```
---
## Appendix
### File Structure

```
zenith-dataplane/
 core/ # Rust core with FFI
 zenith-runtime-cpu/ # CPU runtime (DataLoader, NUMA, SIMD)
 zenith-runtime-gpu/ # GPU runtime (CUDA, TensorRT)
 zenith-scheduler/ # Job scheduler
 sdk-python/ # Python SDK
 bench/ # Benchmark suite
 docs/ # Documentation
 tests/ # Integration tests
```
### Dependencies
**Rust:**
- arrow: 53.x
- parquet: 53.x 
- tokio: 1.x
- parking_lot: 0.12
**Python:**
- pyarrow: 22.x
- torch: 2.x (optional)
---
*Documentation generated for Zenith v0.2.0*
