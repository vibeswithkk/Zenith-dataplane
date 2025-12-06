"""
Zenith PyTorch DataLoader

Drop-in replacement for torch.utils.data.DataLoader with
Zenith's high-performance backend.
"""

from pathlib import Path
from typing import Optional, Union, Callable, Any

try:
    import torch
    from torch.utils.data import DataLoader as TorchDataLoader
    TORCH_AVAILABLE = True
except ImportError:
    TORCH_AVAILABLE = False
    TorchDataLoader = object

from zenith.torch.dataset import ZenithDataset, ZenithMapDataset


class DataLoader:
    """
    High-performance DataLoader powered by Zenith.
    
    This is a convenience wrapper that combines ZenithDataset
    with PyTorch's DataLoader for optimal performance.
    
    Example:
        >>> loader = DataLoader(
        ...     source="path/to/data",
        ...     batch_size=64,
        ...     preprocessing_plugin="augment.wasm"
        ... )
        >>> for batch in loader:
        ...     loss = model.training_step(batch)
    """
    
    def __init__(
        self,
        source: Union[str, Path, ZenithDataset],
        batch_size: int = 32,
        shuffle: bool = True,
        num_workers: int = 4,
        pin_memory: bool = True,
        drop_last: bool = False,
        preprocessing_plugin: Optional[str] = None,
        transform: Optional[Callable] = None,
        target_transform: Optional[Callable] = None,
        label_column: Optional[str] = None,
        prefetch_factor: int = 2,
        persistent_workers: bool = True,
    ):
        """
        Initialize the Zenith DataLoader.
        
        Args:
            source: Data source path or ZenithDataset instance
            batch_size: Samples per batch
            shuffle: Whether to shuffle (only works with ZenithMapDataset)
            num_workers: Parallel data loading workers
            pin_memory: Pin memory for faster GPU transfer
            drop_last: Drop incomplete final batch
            preprocessing_plugin: WASM plugin for fast preprocessing
            transform: Optional feature transform
            target_transform: Optional label transform
            label_column: Name of label column
            prefetch_factor: Batches to prefetch per worker
            persistent_workers: Keep workers alive between epochs
        """
        if not TORCH_AVAILABLE:
            raise ImportError(
                "PyTorch is required for zenith.torch. "
                "Install with: pip install zenith-ai[torch]"
            )
        
        self.batch_size = batch_size
        self.shuffle = shuffle
        self.num_workers = num_workers
        
        # Create dataset if source is a path
        if isinstance(source, (str, Path)):
            if shuffle:
                # Use map-style dataset for shuffling support
                self._dataset = ZenithMapDataset(
                    source=source,
                    preprocessing_plugin=preprocessing_plugin,
                    transform=transform,
                    target_transform=target_transform,
                    label_column=label_column,
                )
            else:
                # Use iterable dataset for streaming
                self._dataset = ZenithDataset(
                    source=source,
                    preprocessing_plugin=preprocessing_plugin,
                    transform=transform,
                    target_transform=target_transform,
                    label_column=label_column,
                )
        else:
            self._dataset = source
        
        # Create underlying PyTorch DataLoader
        loader_kwargs = {
            "batch_size": batch_size,
            "num_workers": num_workers,
            "pin_memory": pin_memory,
            "drop_last": drop_last,
        }
        
        # Shuffle only available for map-style datasets
        if isinstance(self._dataset, ZenithMapDataset):
            loader_kwargs["shuffle"] = shuffle
        
        if num_workers > 0:
            loader_kwargs["prefetch_factor"] = prefetch_factor
            loader_kwargs["persistent_workers"] = persistent_workers
        
        self._loader = TorchDataLoader(self._dataset, **loader_kwargs)
    
    def __iter__(self):
        """Iterate over batches."""
        return iter(self._loader)
    
    def __len__(self):
        """Number of batches."""
        return len(self._loader)
    
    @property
    def dataset(self):
        """Access underlying dataset."""
        return self._dataset
    
    def close(self):
        """Release resources."""
        if hasattr(self._dataset, 'close'):
            self._dataset.close()
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False
    
    def __repr__(self):
        return (
            f"<zenith.torch.DataLoader("
            f"batch_size={self.batch_size}, "
            f"shuffle={self.shuffle}, "
            f"num_workers={self.num_workers})>"
        )
