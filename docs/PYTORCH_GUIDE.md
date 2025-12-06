# PyTorch Integration Guide

This guide covers how to use Zenith AI with PyTorch for high-performance data loading.

## Installation

```bash
pip install zenith-ai[torch]
```

## Quick Start

### Basic DataLoader

Replace your standard PyTorch DataLoader with Zenith's accelerated version:

```python
import zenith.torch as zt

# Create a Zenith-accelerated DataLoader
loader = zt.DataLoader(
    source="path/to/training_data.parquet",
    batch_size=64,
    shuffle=True,
    num_workers=4
)

# Use in your training loop
for batch in loader:
    outputs = model(batch)
    loss = criterion(outputs, targets)
    loss.backward()
    optimizer.step()
```

### With Preprocessing Plugin

For maximum performance, use WASM plugins for preprocessing:

```python
import zenith.torch as zt

loader = zt.DataLoader(
    source="s3://bucket/imagenet",
    batch_size=64,
    preprocessing_plugin="image_resize.wasm",
    transform=lambda x: x / 255.0,  # Additional Python transforms
)

for images, labels in loader:
    outputs = model(images)
    # ... training loop
```

## Dataset Types

### ZenithDataset (Iterable)

Best for:
- Large datasets that don't fit in memory
- Streaming from remote sources (S3, GCS)
- When you don't need random access

```python
from zenith.torch import ZenithDataset
from torch.utils.data import DataLoader

dataset = ZenithDataset(
    source="s3://bucket/huge-dataset",
    preprocessing_plugin="tokenizer.wasm"
)

# Note: shuffle=False for IterableDataset
loader = DataLoader(dataset, batch_size=32)
```

### ZenithMapDataset (Map-style)

Best for:
- Smaller datasets
- When you need shuffling
- Random access patterns

```python
from zenith.torch import ZenithMapDataset
from torch.utils.data import DataLoader

dataset = ZenithMapDataset(
    source="path/to/data.parquet",
    label_column="target"
)

loader = DataLoader(dataset, batch_size=32, shuffle=True)
```

## Performance Tips

### 1. Use Multiple Workers

```python
loader = zt.DataLoader(
    source="data/",
    batch_size=64,
    num_workers=8,  # Match CPU cores
    prefetch_factor=2,
    persistent_workers=True
)
```

### 2. Pin Memory for GPU Training

```python
loader = zt.DataLoader(
    source="data/",
    batch_size=64,
    pin_memory=True  # Faster CPU->GPU transfer
)
```

### 3. Use WASM Preprocessing

Move expensive operations to WASM:

```python
# Slow: Python-based resize
transform = transforms.Resize((224, 224))  # Bottleneck!

# Fast: WASM-based resize
loader = zt.DataLoader(
    source="images/",
    preprocessing_plugin="image_ops.wasm"  # 10x faster
)
```

## Distributed Training

Zenith works with PyTorch's DistributedDataParallel:

```python
import torch.distributed as dist
from torch.nn.parallel import DistributedDataParallel
import zenith.torch as zt

# Initialize distributed
dist.init_process_group("nccl")

# Create per-rank loader
loader = zt.DataLoader(
    source="data/",
    batch_size=64 // dist.get_world_size(),
)

# Wrap model
model = DistributedDataParallel(model)

for batch in loader:
    # Distributed training loop
    pass
```

## Complete Training Example

```python
import torch
import torch.nn as nn
import zenith.torch as zt

# Model
model = nn.Sequential(
    nn.Linear(784, 256),
    nn.ReLU(),
    nn.Linear(256, 10)
)
model = model.cuda()

# Zenith DataLoader
train_loader = zt.DataLoader(
    source="data/train.parquet",
    batch_size=128,
    shuffle=True,
    preprocessing_plugin="normalize.wasm",
    label_column="label",
    pin_memory=True,
    num_workers=4
)

# Training
optimizer = torch.optim.Adam(model.parameters())
criterion = nn.CrossEntropyLoss()

for epoch in range(10):
    for features, labels in train_loader:
        features = features.cuda()
        labels = labels.cuda()
        
        optimizer.zero_grad()
        outputs = model(features)
        loss = criterion(outputs, labels)
        loss.backward()
        optimizer.step()
    
    print(f"Epoch {epoch+1} complete")
```

## Troubleshooting

### Import Error: PyTorch not found

```bash
pip install zenith-ai[torch]
# or
pip install torch zenith-ai
```

### Slow Performance

1. Increase `num_workers`
2. Use WASM preprocessing plugins
3. Enable `pin_memory=True` for GPU training
4. Check that data is on fast storage (NVMe SSD)

### Memory Issues

Use `ZenithDataset` (IterableDataset) instead of `ZenithMapDataset` for large datasets.
