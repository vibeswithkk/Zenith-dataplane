# Zenith Tools

Collection of utilities for working with Zenith Data Plane.

## Tools

### 1. **benchmark.py** - Performance Testing
```bash
python3 tools/benchmark.py
```
Tests ingestion throughput and latency.

### 2. **inspector.py** - Engine Monitoring
```bash
# One-shot status
python3 tools/inspector.py

# Continuous monitoring
python3 tools/inspector.py watch
```
Real-time engine status and plugin monitoring via Admin API.

### 3. **build_plugin.sh** - WASM Plugin Builder
```bash
./tools/build_plugin.sh plugins/simple_filter
```
Automated plugin compilation with proper WASM target.

## Requirements

- Python 3.8+
- requests (for inspector)
- pyarrow, numpy (for benchmark)
- Rust toolchain with wasm32-wasip1 target

## Installation

```bash
pip install requests pyarrow numpy
rustup target add wasm32-wasip1
```

## Usage Examples

### Running Performance Benchmark
```bash
# Start engine first
./scripts/start_all.sh

# In another terminal
python3 tools/benchmark.py
```

### Monitoring Engine
```bash
# Continuous watch mode
python3 tools/inspector.py watch
```

### Building Custom Plugins
```bash
./tools/build_plugin.sh plugins/my_custom_filter
```

## Output Examples

**Inspector:**
```
============================================================
Zenith Engine Status - 2025-12-07 03:50:00
============================================================
Status:       RUNNING
Buffer Size:  1,024
Plugins:      2
============================================================
```

**Benchmark:**
```
[OK] Ingested 1,000,000 events in 0.25s
[STATS] Throughput: 4,000,000 events/sec
[TIME] Latency: 0.25 ms/batch
```
