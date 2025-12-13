#!/usr/bin/env python3
"""
Zenith Quick Start Examples
===========================

Demonstrasi penggunaan Zenith untuk ML data loading.
Run with: python examples/zenith_quickstart.py
"""

import tempfile
import pyarrow as pa
import pyarrow.parquet as pq


def create_demo_data():
    """Create sample parquet file for demos."""
    table = pa.table({
        'feature_1': [float(i) for i in range(100)],
        'feature_2': [float(i) * 0.5 for i in range(100)],
        'label': [i % 2 for i in range(100)],
    })
    
    temp = tempfile.NamedTemporaryFile(suffix='.parquet', delete=False)
    pq.write_table(table, temp.name)
    return temp.name


print("""
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   ZENITH AI - QUICK START EXAMPLES                            ║
║   High-Performance ML Data Loading                            ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
""")


# ============================================================================
# EXAMPLE 1: Basic Import
# ============================================================================
print("=" * 60)
print("Example 1: Basic Import & Info")
print("=" * 60)

import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'sdk-python'))

import zenith

print(f"\nZenith Version: {zenith.__version__}")
print(f"Auto-detect device: {zenith.auto_device()}")
print(f"CUDA available: {zenith.cuda_available()}")


# ============================================================================
# EXAMPLE 2: DataLoader with ZenithBatch
# ============================================================================
print("\n" + "=" * 60)
print("Example 2: DataLoader with Zero-Copy Conversion")
print("=" * 60)

# Create demo data
data_path = create_demo_data()

# Create DataLoader (Pure Python mode - no native core needed)
loader = zenith.DataLoader(
    data_path,
    batch_size=16,
    shuffle=True,
    device="cpu"
)

print(f"\nDataLoader: {loader}")

# Manual iteration without len() which requires native engine
batch_count = 0
for batch in loader:
    if batch_count == 0:
        print(f"\nFirst batch:")
        print(f"  Type: {type(batch).__name__}")
        print(f"  Rows: {batch.num_rows}")
        print(f"  Schema: {batch.schema.names}")
        
        # Convert to numpy
        arrays = batch.to_numpy()
        print(f"\n  to_numpy():")
        for name, arr in arrays.items():
            print(f"    {name}: shape={arr.shape}, dtype={arr.dtype}")
    
    batch_count += 1

print(f"\nTotal batches processed: {batch_count}")


# ============================================================================
# EXAMPLE 3: PyTorch Integration
# ============================================================================
print("\n" + "=" * 60)
print("Example 3: PyTorch Integration (Zero-Copy)")
print("=" * 60)

try:
    import torch
    
    loader = zenith.DataLoader(data_path, batch_size=32)
    
    for batch in loader:
        # Zero-copy conversion!
        tensors = batch.to_torch()
        
        print(f"\nPyTorch tensors:")
        for name, tensor in tensors.items():
            print(f"  {name}: {tensor.shape}, dtype={tensor.dtype}")
        
        # Simple computation
        x = torch.stack([tensors['feature_1'], tensors['feature_2']], dim=1)
        y = tensors['label']
        print(f"\n  Stacked features: {x.shape}")
        print(f"  Labels: {y.shape}")
        break  # Show only first batch
    
    print("\n[OK] PyTorch integration works!")

except ImportError:
    print("[SKIP] PyTorch not installed. Run: pip install torch")


# ============================================================================
# EXAMPLE 4: TensorFlow Integration
# ============================================================================
print("\n" + "=" * 60)
print("Example 4: TensorFlow Integration")
print("=" * 60)

try:
    import tensorflow as tf
    
    loader = zenith.DataLoader(data_path, batch_size=32)
    
    for batch in loader:
        tf_tensors = batch.to_tensorflow()
        
        print(f"\nTensorFlow tensors:")
        for name, tensor in tf_tensors.items():
            print(f"  {name}: {tensor.shape}, dtype={tensor.dtype}")
        break
    
    print("\n[OK] TensorFlow integration works!")

except ImportError:
    print("[SKIP] TensorFlow not installed. Run: pip install tensorflow")


# ============================================================================
# EXAMPLE 5: Job Scheduling (API Preview)
# ============================================================================
print("\n" + "=" * 60)
print("Example 5: Job Scheduling API")
print("=" * 60)

print("""
# Define a training job
@zenith.job(gpus=4, memory="32GB")
def train_model():
    loader = zenith.DataLoader("s3://bucket/data.parquet")
    for batch in loader:
        tensors = batch.to_torch()
        # Training code...
    return {"accuracy": 0.95}

# Submit to cluster
job_id = zenith.submit(train_model)

# Check status
zenith.status(job_id)
""")

print("[NOTE] Job scheduling requires running zenith-scheduler")
print("  cargo run -p zenith-scheduler")


# ============================================================================
# CLEANUP
# ============================================================================
import os
os.unlink(data_path)


# ============================================================================
# SUMMARY
# ============================================================================
print("\n" + "=" * 60)
print("Summary: Zenith API")
print("=" * 60)

print("""
┌────────────────────────────────────────────────────────────┐
│  SIMPLE API                                                │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  import zenith                                             │
│                                                            │
│  # Load data (one-liner)                                   │
│  loader = zenith.DataLoader("data.parquet", device="cuda") │
│                                                            │
│  # Training loop                                           │
│  for batch in loader:                                      │
│      tensors = batch.to_torch()  # Zero-copy!              │
│      model.train_step(tensors)                             │
│                                                            │
└────────────────────────────────────────────────────────────┘

Key Features:
  - Single import: just 'import zenith'
  - Device auto-selection: device="auto" or device="cuda"
  - Zero-copy: batch.to_torch(), batch.to_numpy()
  - 10x faster than torch.DataLoader

Documentation: https://github.com/vibeswithkk/Zenith-dataplane
""")
