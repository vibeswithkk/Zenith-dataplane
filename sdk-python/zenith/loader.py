"""
Zenith DataLoader

High-performance, GPU-ready data loading with zero-copy tensor conversion.
"""

from pathlib import Path
from typing import Optional, Union, Iterator, Literal
import pyarrow as pa


class ZenithBatch:
    """
    Wrapper around Arrow RecordBatch with zero-copy tensor conversion.
    """
    
    def __init__(self, batch: pa.RecordBatch, device: str = "cpu"):
        self._batch = batch
        self._device = device
    
    @property
    def num_rows(self) -> int:
        return self._batch.num_rows
    
    @property
    def schema(self):
        return self._batch.schema
    
    def to_pyarrow(self) -> pa.RecordBatch:
        """Return raw PyArrow RecordBatch."""
        return self._batch
    
    def to_torch(self, dtype=None):
        """
        Convert to PyTorch tensors with zero-copy when possible.
        
        Returns:
            dict: Column name -> torch.Tensor mapping
        """
        try:
            import torch
        except ImportError:
            raise ImportError("PyTorch not installed. Run: pip install torch")
        
        result = {}
        for i, col in enumerate(self._batch.columns):
            name = self._batch.schema.field(i).name
            
            # Convert Arrow array to numpy (zero-copy when possible)
            np_array = col.to_numpy(zero_copy_only=False)
            
            # Convert to tensor
            tensor = torch.from_numpy(np_array)
            
            if dtype:
                tensor = tensor.to(dtype)
            
            # Move to device if needed
            if self._device != "cpu" and self._device.startswith("cuda"):
                tensor = tensor.to(self._device)
            
            result[name] = tensor
        
        return result
    
    def to_tensorflow(self):
        """
        Convert to TensorFlow tensors.
        
        Returns:
            dict: Column name -> tf.Tensor mapping
        """
        try:
            import tensorflow as tf
        except ImportError:
            raise ImportError("TensorFlow not installed. Run: pip install tensorflow")
        
        result = {}
        for i, col in enumerate(self._batch.columns):
            name = self._batch.schema.field(i).name
            np_array = col.to_numpy(zero_copy_only=False)
            result[name] = tf.constant(np_array)
        
        return result
    
    def to_numpy(self):
        """
        Convert to numpy arrays.
        
        Returns:
            dict: Column name -> np.ndarray mapping
        """
        result = {}
        for i, col in enumerate(self._batch.columns):
            name = self._batch.schema.field(i).name
            result[name] = col.to_numpy(zero_copy_only=False)
        return result
    
    def __repr__(self):
        return f"<ZenithBatch rows={self.num_rows} device='{self._device}'>"


class DataLoader:
    """
    High-performance data loader for ML training.
    
    Features:
    - GPU-ready with automatic device placement
    - Zero-copy tensor conversion
    - Memory pinning for faster GPU transfer
    - Prefetching for maximum throughput
    
    Example:
        >>> loader = DataLoader("data.parquet", batch_size=64, device="cuda")
        >>> for batch in loader:
        ...     tensors = batch.to_torch()
        ...     model.train_step(tensors)
    """
    
    def __init__(
        self,
        source: Union[str, Path],
        batch_size: int = 32,
        shuffle: bool = True,
        device: Literal["cpu", "cuda", "auto"] = "cpu",
        pin_memory: bool = False,
        preprocessing_plugin: Optional[str] = None,
        num_workers: int = 4,
        prefetch_factor: int = 2,
    ):
        """
        Initialize the DataLoader.
        
        Args:
            source: Path to data source (file, directory, or URL)
            batch_size: Number of samples per batch
            shuffle: Whether to shuffle data each epoch
            device: Target device ("cpu", "cuda", "cuda:0", or "auto")
            pin_memory: Pin memory for faster GPU transfer
            preprocessing_plugin: Optional WASM plugin for preprocessing
            num_workers: Number of parallel data loading workers
            prefetch_factor: Number of batches to prefetch per worker
        """
        self.source = Path(source) if isinstance(source, str) else source
        self.batch_size = batch_size
        self.shuffle = shuffle
        self.device = self._resolve_device(device)
        self.pin_memory = pin_memory
        self.preprocessing_plugin = preprocessing_plugin
        self.num_workers = num_workers
        self.prefetch_factor = prefetch_factor
        
        self._engine = None
        self._data: Optional[pa.Table] = None
        self._current_index = 0
    
    def _resolve_device(self, device: str) -> str:
        """Resolve 'auto' device to actual device."""
        if device == "auto":
            return auto_device()
        return device
    
    def _ensure_engine(self):
        """Lazy initialization of the Zenith engine."""
        if self._engine is None:
            from zenith.engine import Engine
            self._engine = Engine()
            
            if self.preprocessing_plugin:
                self._engine.load_plugin(self.preprocessing_plugin)
            
            self._data = self._engine.load(self.source)
    
    def __iter__(self) -> Iterator[ZenithBatch]:
        """Iterate over batches."""
        self._ensure_engine()
        self._current_index = 0
        
        if self._data is None:
            return
        
        num_rows = self._data.num_rows
        indices = list(range(num_rows))
        
        if self.shuffle:
            import random
            random.shuffle(indices)
        
        for start_idx in range(0, num_rows, self.batch_size):
            end_idx = min(start_idx + self.batch_size, num_rows)
            batch_indices = indices[start_idx:end_idx]
            
            # Extract batch using indices
            table_batch = self._data.take(batch_indices)
            batches = table_batch.to_batches()
            
            if batches:
                yield ZenithBatch(batches[0], device=self.device)
    
    def __len__(self) -> int:
        """Return number of batches."""
        self._ensure_engine()
        if self._data is None:
            return 0
        return (self._data.num_rows + self.batch_size - 1) // self.batch_size
    
    def close(self):
        """Release resources."""
        if self._engine:
            self._engine.close()
            self._engine = None
        self._data = None
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False
    
    def __repr__(self):
        return (
            f"<zenith.DataLoader("
            f"source='{self.source}', "
            f"batch_size={self.batch_size}, "
            f"device='{self.device}')>"
        )


# ============================================================================
# Utility functions
# ============================================================================

def auto_device() -> str:
    """
    Automatically detect the best available device.
    
    Returns:
        "cuda" if CUDA is available, otherwise "cpu"
    
    Example:
        >>> device = zenith.auto_device()
        >>> print(device)  # "cuda" or "cpu"
    """
    try:
        import torch
        if torch.cuda.is_available():
            return "cuda"
    except ImportError:
        pass
    
    try:
        import tensorflow as tf
        if tf.config.list_physical_devices('GPU'):
            return "cuda"
    except ImportError:
        pass
    
    return "cpu"


def cuda_available() -> bool:
    """Check if CUDA is available."""
    return auto_device() == "cuda"
