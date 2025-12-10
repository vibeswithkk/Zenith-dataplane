"""
Zenith PyTorch Dataset

A PyTorch Dataset backed by Zenith's high-performance data loading.
"""

from pathlib import Path
from typing import Optional, Union, Callable, Any, Dict

try:
    import torch
    from torch.utils.data import Dataset, IterableDataset
    TORCH_AVAILABLE = True
except ImportError:
    TORCH_AVAILABLE = False
    Dataset = object
    IterableDataset = object

import pyarrow as pa


class ZenithDataset(IterableDataset if TORCH_AVAILABLE else object):  # type: ignore[misc]
    """
    PyTorch-compatible Dataset using Zenith for data loading.
    
    This dataset provides streaming access to large datasets
    without loading everything into memory, while maintaining
    PyTorch compatibility.
    
    Example:
        >>> dataset = ZenithDataset(
        ...     source="s3://bucket/training-data",
        ...     preprocessing_plugin="tokenizer.wasm"
        ... )
        >>> loader = torch.utils.data.DataLoader(dataset, batch_size=64)
        >>> for batch in loader:
        ...     outputs = model(batch)
    """
    
    def __init__(
        self,
        source: Union[str, Path],
        preprocessing_plugin: Optional[str] = None,
        transform: Optional[Callable] = None,
        target_transform: Optional[Callable] = None,
        columns: Optional[list] = None,
        label_column: Optional[str] = None,
    ):
        """
        Initialize the ZenithDataset.
        
        Args:
            source: Path to data source (local path, S3 URL, etc.)
            preprocessing_plugin: WASM plugin for fast preprocessing
            transform: Optional transform to apply to features
            target_transform: Optional transform to apply to labels
            columns: Specific columns to load (None = all)
            label_column: Name of the label column
        """
        if not TORCH_AVAILABLE:
            raise ImportError(
                "PyTorch is required for zenith.torch. "
                "Install with: pip install zenith-ai[torch]"
            )
        
        super().__init__()
        
        self.source = Path(source) if isinstance(source, str) else source
        self.preprocessing_plugin = preprocessing_plugin
        self.transform = transform
        self.target_transform = target_transform
        self.columns = columns
        self.label_column = label_column
        
        self._engine = None
        self._data: Optional[pa.Table] = None
    
    def _ensure_loaded(self):
        """Lazy load data using Zenith engine."""
        if self._engine is None:
            from zenith.engine import Engine
            self._engine = Engine()
            
            if self.preprocessing_plugin:
                self._engine.load_plugin(self.preprocessing_plugin)
            
            self._data = self._engine.load(self.source)
            
            if self.columns:
                self._data = self._data.select(self.columns)
    
    def __iter__(self):
        """Iterate over samples."""
        self._ensure_loaded()
        
        if self._data is None:
            return
        
        for i in range(self._data.num_rows):
            row = {col: self._data.column(col)[i].as_py() 
                   for col in self._data.column_names}
            
            # Separate features and label if specified
            if self.label_column and self.label_column in row:
                label = row.pop(self.label_column)
                features = row
            else:
                features = row
                label = None
            
            # Convert to tensors
            if len(features) == 1:
                # Single feature column
                features = torch.tensor(list(features.values())[0])
            else:
                # Multiple columns - return as dict of tensors
                features = {k: torch.tensor(v) for k, v in features.items()}
            
            if self.transform:
                features = self.transform(features)
            
            if label is not None:
                label = torch.tensor(label)
                if self.target_transform:
                    label = self.target_transform(label)
                yield features, label
            else:
                yield features
    
    def __len__(self) -> int:
        """Return dataset size."""
        self._ensure_loaded()
        return self._data.num_rows if self._data else 0
    
    def close(self):
        """Release resources."""
        if self._engine:
            self._engine.close()
            self._engine = None
        self._data = None
    
    def __repr__(self):
        return (
            f"<zenith.torch.ZenithDataset("
            f"source='{self.source}', "
            f"plugin={self.preprocessing_plugin})>"
        )


class ZenithMapDataset(Dataset if TORCH_AVAILABLE else object):  # type: ignore[misc]
    """
    Map-style PyTorch Dataset for random access.
    
    Use this when you need random access to samples (e.g., for shuffling
    within PyTorch's DataLoader).
    
    Note: This loads the entire dataset into memory. For large datasets,
    use ZenithDataset (IterableDataset) instead.
    """
    
    def __init__(
        self,
        source: Union[str, Path],
        preprocessing_plugin: Optional[str] = None,
        transform: Optional[Callable] = None,
        target_transform: Optional[Callable] = None,
        label_column: Optional[str] = None,
    ):
        if not TORCH_AVAILABLE:
            raise ImportError(
                "PyTorch is required for zenith.torch. "
                "Install with: pip install zenith-ai[torch]"
            )
        
        super().__init__()
        
        self.source = Path(source) if isinstance(source, str) else source
        self.preprocessing_plugin = preprocessing_plugin
        self.transform = transform
        self.target_transform = target_transform
        self.label_column = label_column
        
        # Load data immediately for random access
        from zenith.engine import Engine
        self._engine = Engine()
        
        if self.preprocessing_plugin:
            self._engine.load_plugin(self.preprocessing_plugin)
        
        self._data = self._engine.load(self.source)
    
    def __getitem__(self, idx: int):
        """Get sample by index."""
        row = {col: self._data.column(col)[idx].as_py() 
               for col in self._data.column_names}
        
        if self.label_column and self.label_column in row:
            label = row.pop(self.label_column)
            features = row
        else:
            features = row
            label = None
        
        # Convert to tensors
        if len(features) == 1:
            features = torch.tensor(list(features.values())[0])
        else:
            features = {k: torch.tensor(v) for k, v in features.items()}
        
        if self.transform:
            features = self.transform(features)
        
        if label is not None:
            label = torch.tensor(label)
            if self.target_transform:
                label = self.target_transform(label)
            return features, label
        
        return features
    
    def __len__(self) -> int:
        return self._data.num_rows if self._data else 0
    
    def close(self):
        if self._engine:
            self._engine.close()
            self._engine = None
        self._data = None
