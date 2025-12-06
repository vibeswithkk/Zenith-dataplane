"""
Zenith AI: High-Performance Data Infrastructure for Machine Learning

This package provides blazing-fast data loading and preprocessing
for AI/ML training pipelines, with native support for PyTorch,
TensorFlow, and other frameworks.

Example:
    >>> import zenith
    >>> engine = zenith.Engine()
    >>> data = engine.load("path/to/dataset")
"""

__version__ = "0.1.0"
__author__ = "Zenith Contributors"

# Try to import native Rust extension first
_NATIVE_AVAILABLE = False
try:
    from zenith._core import Engine as NativeEngine
    from zenith._core import version as native_version
    from zenith._core import is_available
    _NATIVE_AVAILABLE = is_available()
except ImportError:
    _NATIVE_AVAILABLE = False

# Import Python fallback implementations
from zenith.engine import Engine as PythonEngine
from zenith.loader import DataLoader

# Use native if available, otherwise fallback to Python
if _NATIVE_AVAILABLE:
    Engine = NativeEngine
else:
    Engine = PythonEngine

# Lazy imports for framework-specific adapters
def __getattr__(name):
    if name == "torch":
        from zenith import torch as _torch
        return _torch
    elif name == "tensorflow":
        from zenith import tensorflow as _tensorflow
        return _tensorflow
    elif name == "native_available":
        return _NATIVE_AVAILABLE
    raise AttributeError(f"module 'zenith' has no attribute '{name}'")

__all__ = [
    "Engine",
    "DataLoader",
    "__version__",
    "native_available",
]
