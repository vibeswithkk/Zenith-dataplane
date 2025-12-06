"""
Zenith TensorFlow Integration

Provides tf.data compatible interface with Zenith's
high-performance data loading capabilities.

Example:
    >>> import zenith.tensorflow as ztf
    >>> dataset = ztf.ZenithDataset("path/to/data")
    >>> dataset = dataset.batch(32).prefetch(tf.data.AUTOTUNE)
    >>> model.fit(dataset, epochs=10)
"""

from zenith.tensorflow.dataset import ZenithDataset

__all__ = [
    "ZenithDataset",
]
