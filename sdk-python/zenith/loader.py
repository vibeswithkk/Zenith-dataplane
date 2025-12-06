"""
Zenith DataLoader

Framework-agnostic high-performance data loading.
"""

from pathlib import Path
from typing import Optional, Union, Iterator, Any, List
import pyarrow as pa


class DataLoader:
    """
    High-performance data loader for ML training.
    
    Provides an iterator interface compatible with standard
    ML training loops while using Zenith's Rust core for speed.
    
    Example:
        >>> loader = DataLoader("path/to/data", batch_size=64)
        >>> for batch in loader:
        ...     model.train_step(batch)
    """
    
    def __init__(
        self,
        source: Union[str, Path],
        batch_size: int = 32,
        shuffle: bool = True,
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
            preprocessing_plugin: Optional WASM plugin for preprocessing
            num_workers: Number of parallel data loading workers
            prefetch_factor: Number of batches to prefetch per worker
        """
        self.source = Path(source) if isinstance(source, str) else source
        self.batch_size = batch_size
        self.shuffle = shuffle
        self.preprocessing_plugin = preprocessing_plugin
        self.num_workers = num_workers
        self.prefetch_factor = prefetch_factor
        
        self._engine = None
        self._data: Optional[pa.Table] = None
        self._current_index = 0
    
    def _ensure_engine(self):
        """Lazy initialization of the Zenith engine."""
        if self._engine is None:
            from zenith.engine import Engine
            self._engine = Engine()
            
            if self.preprocessing_plugin:
                self._engine.load_plugin(self.preprocessing_plugin)
            
            self._data = self._engine.load(self.source)
    
    def __iter__(self) -> Iterator[pa.RecordBatch]:
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
            batch = self._data.take(batch_indices)
            yield batch.to_batches()[0] if batch.to_batches() else None
    
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
            f"shuffle={self.shuffle})>"
        )
