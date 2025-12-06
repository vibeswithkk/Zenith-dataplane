"""
Zenith TensorFlow Dataset

A tf.data.Dataset backed by Zenith's high-performance data loading.
"""

from pathlib import Path
from typing import Optional, Union, Tuple, Any

try:
    import tensorflow as tf
    TF_AVAILABLE = True
except ImportError:
    TF_AVAILABLE = False


class ZenithDataset:
    """
    TensorFlow-compatible Dataset using Zenith for data loading.
    
    This creates a tf.data.Dataset that streams data through
    Zenith's high-performance Rust core.
    
    Example:
        >>> dataset = ZenithDataset(
        ...     source="/data/imagenet",
        ...     preprocessing_plugin="augment.wasm"
        ... )
        >>> dataset = dataset.batch(32).prefetch(tf.data.AUTOTUNE)
        >>> model.fit(dataset, epochs=10)
    """
    
    def __init__(
        self,
        source: Union[str, Path],
        preprocessing_plugin: Optional[str] = None,
        label_column: Optional[str] = None,
        output_signature: Optional[Tuple] = None,
    ):
        """
        Initialize the ZenithDataset for TensorFlow.
        
        Args:
            source: Path to data source
            preprocessing_plugin: WASM plugin for preprocessing
            label_column: Name of label column
            output_signature: TensorFlow output signature spec
        """
        if not TF_AVAILABLE:
            raise ImportError(
                "TensorFlow is required for zenith.tensorflow. "
                "Install with: pip install zenith-ai[tensorflow]"
            )
        
        self.source = Path(source) if isinstance(source, str) else source
        self.preprocessing_plugin = preprocessing_plugin
        self.label_column = label_column
        self.output_signature = output_signature
        
        self._engine = None
        self._data = None
        self._tf_dataset = None
    
    def _ensure_loaded(self):
        """Load data and create tf.data.Dataset."""
        if self._engine is None:
            from zenith.engine import Engine
            self._engine = Engine()
            
            if self.preprocessing_plugin:
                self._engine.load_plugin(self.preprocessing_plugin)
            
            self._data = self._engine.load(self.source)
            self._create_tf_dataset()
    
    def _create_tf_dataset(self):
        """Create the underlying tf.data.Dataset."""
        def generator():
            for i in range(self._data.num_rows):
                row = {col: self._data.column(col)[i].as_py() 
                       for col in self._data.column_names}
                
                if self.label_column and self.label_column in row:
                    label = row.pop(self.label_column)
                    features = row
                    yield features, label
                else:
                    yield row
        
        # Infer output signature if not provided
        if self.output_signature:
            signature = self.output_signature
        else:
            # Default to string -> float32 mapping
            sample_row = {col: self._data.column(col)[0].as_py() 
                         for col in self._data.column_names}
            
            if self.label_column:
                label = sample_row.pop(self.label_column)
                feature_spec = {k: tf.TensorSpec(shape=(), dtype=tf.float32) 
                               for k in sample_row.keys()}
                label_spec = tf.TensorSpec(shape=(), dtype=tf.int64)
                signature = (feature_spec, label_spec)
            else:
                signature = {k: tf.TensorSpec(shape=(), dtype=tf.float32) 
                            for k in sample_row.keys()}
        
        self._tf_dataset = tf.data.Dataset.from_generator(
            generator,
            output_signature=signature
        )
    
    def batch(self, batch_size: int):
        """Batch the dataset."""
        self._ensure_loaded()
        self._tf_dataset = self._tf_dataset.batch(batch_size)
        return self
    
    def prefetch(self, buffer_size):
        """Prefetch batches."""
        self._ensure_loaded()
        self._tf_dataset = self._tf_dataset.prefetch(buffer_size)
        return self
    
    def shuffle(self, buffer_size: int):
        """Shuffle the dataset."""
        self._ensure_loaded()
        self._tf_dataset = self._tf_dataset.shuffle(buffer_size)
        return self
    
    def map(self, map_func, num_parallel_calls=None):
        """Apply a transformation function."""
        self._ensure_loaded()
        self._tf_dataset = self._tf_dataset.map(
            map_func, 
            num_parallel_calls=num_parallel_calls
        )
        return self
    
    def repeat(self, count=None):
        """Repeat the dataset."""
        self._ensure_loaded()
        self._tf_dataset = self._tf_dataset.repeat(count)
        return self
    
    def take(self, count: int):
        """Take first N elements."""
        self._ensure_loaded()
        self._tf_dataset = self._tf_dataset.take(count)
        return self
    
    def __iter__(self):
        """Iterate over the dataset."""
        self._ensure_loaded()
        return iter(self._tf_dataset)
    
    def as_numpy_iterator(self):
        """Return numpy iterator."""
        self._ensure_loaded()
        return self._tf_dataset.as_numpy_iterator()
    
    @property
    def element_spec(self):
        """Return element specification."""
        self._ensure_loaded()
        return self._tf_dataset.element_spec
    
    def close(self):
        """Release resources."""
        if self._engine:
            self._engine.close()
            self._engine = None
        self._data = None
        self._tf_dataset = None
    
    def __repr__(self):
        return (
            f"<zenith.tensorflow.ZenithDataset("
            f"source='{self.source}')>"
        )
