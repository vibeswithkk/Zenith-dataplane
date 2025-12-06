#!/usr/bin/env python3
"""
Zenith AI - Basic Usage Example

Demonstrates the core Zenith engine functionality:
1. Initializing the engine
2. Loading a preprocessing plugin
3. Publishing data with zero-copy

Requirements:
    pip install zenith-ai pyarrow numpy
"""

import time
import pyarrow as pa
import numpy as np

# Add sdk-python to path for development
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'sdk-python'))

from zenith import Engine


def create_sample_data(batch_size: int = 1000) -> pa.RecordBatch:
    """Generate sample data for benchmarking."""
    return pa.RecordBatch.from_arrays([
        pa.array(np.arange(batch_size, dtype=np.int64)),
        pa.array(np.random.randn(batch_size)),
        pa.array([f"item_{i}" for i in range(batch_size)]),
    ], names=['id', 'value', 'name'])


def main():
    print("=" * 60)
    print("Zenith AI - Basic Engine Example")
    print("=" * 60)
    
    # Initialize engine
    print("\n[1] Initializing Zenith Engine...")
    try:
        engine = Engine(buffer_size=4096)
        print(f"    {engine}")
    except RuntimeError as e:
        print(f"    [ERROR] Could not initialize engine: {e}")
        print("    Make sure to build the core first: cargo build --release")
        return
    
    # Load plugin (optional)
    plugin_path = os.path.join(os.path.dirname(__file__), "filter.wasm")
    if os.path.exists(plugin_path):
        print(f"\n[2] Loading plugin: {plugin_path}")
        engine.load_plugin(plugin_path)
        print(f"    Loaded plugins: {engine.plugins}")
    else:
        print(f"\n[2] Skipping plugin (not found at {plugin_path})")
    
    # Publish data
    print("\n[3] Publishing sample data...")
    batch = create_sample_data(1000)
    
    start = time.perf_counter()
    num_batches = 100
    
    for i in range(num_batches):
        engine.publish(batch, source_id=1, seq_no=i)
    
    elapsed = time.perf_counter() - start
    total_records = 1000 * num_batches
    
    print(f"    Published {num_batches} batches ({total_records:,} records)")
    print(f"    Time: {elapsed*1000:.2f}ms")
    print(f"    Throughput: {total_records/elapsed:,.0f} records/sec")
    
    # Cleanup
    engine.close()
    print("\n[4] Engine closed successfully")
    print("=" * 60)


if __name__ == "__main__":
    main()
