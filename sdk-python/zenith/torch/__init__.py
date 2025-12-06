"""
Zenith PyTorch Integration

Provides native PyTorch DataLoader compatibility with Zenith's
high-performance data loading capabilities.

Example:
    >>> import zenith.torch as zt
    >>> dataset = zt.ZenithDataset("path/to/data")
    >>> loader = zt.DataLoader(dataset, batch_size=64)
    >>> for batch in loader:
    ...     model.train_step(batch)
"""

from zenith.torch.dataset import ZenithDataset
from zenith.torch.dataloader import DataLoader

__all__ = [
    "ZenithDataset",
    "DataLoader",
]
