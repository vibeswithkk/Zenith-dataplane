"""
Zenith AI: High-Performance ML Infrastructure

Fast data loading, efficient preprocessing, and simple job scheduling
for machine learning pipelines.

Quick Start
-----------

1. Drop-in DataLoader replacement:

    # Just change this one import!
    from zenith.torch import DataLoader
    
    loader = DataLoader("data.parquet", batch_size=64)
    for batch in loader:
        model.train_step(batch)

2. Direct usage:

    import zenith
    
    # Load data fast
    data = zenith.load("data.parquet")
    
    # Or use DataLoader
    loader = zenith.DataLoader("data.parquet", batch_size=64)

3. Job scheduling (SLURM alternative):

    import zenith
    
    @zenith.job(gpus=4)
    def train():
        ...
    
    zenith.submit(train)

For more details, see: https://github.com/yourusername/zenith
"""

__version__ = "0.3.2"
__author__ = "Wahyu Ardiansyah"

# ============================================================================
# Core imports
# ============================================================================

# Try to import native Rust extension first
_NATIVE_AVAILABLE = False
try:
    from zenith._core import Engine as NativeEngine
    from zenith._core import is_available
    _NATIVE_AVAILABLE = is_available()
except ImportError:
    _NATIVE_AVAILABLE = False

# Import Python implementations
from zenith.engine import Engine as PythonEngine
from zenith.loader import DataLoader, ZenithBatch, auto_device, cuda_available

# Use native if available, otherwise fallback to Python
if _NATIVE_AVAILABLE:
    Engine = NativeEngine
else:
    Engine = PythonEngine

# ============================================================================
# Convenience functions (the "batteries included" API)
# ============================================================================

_default_engine = None

def _get_engine():
    """Get or create the default engine instance."""
    global _default_engine
    if _default_engine is None:
        # Always use PythonEngine for load() since it has parquet/csv support
        _default_engine = PythonEngine()
    return _default_engine


def load(source, **kwargs):
    """
    Load data from a source.
    
    This is the simplest way to load data with Zenith.
    
    Args:
        source: Path to data file (parquet, csv, arrow)
        **kwargs: Additional options
        
    Returns:
        PyArrow Table with the loaded data
        
    Example:
        >>> import zenith
        >>> data = zenith.load("train.parquet")
        >>> print(f"Loaded {data.num_rows} rows")
    """
    engine = _get_engine()
    return engine.load(source, **kwargs)


def info():
    """
    Print Zenith system information.
    
    Example:
        >>> import zenith
        >>> zenith.info()
    """
    print(f"Zenith v{__version__}")
    print(f"Native core: {'✓ Available' if _NATIVE_AVAILABLE else '✗ Using Python fallback'}")
    print(f"Engine: {Engine.__name__}")


# ============================================================================
# Job scheduling (integrated with Rust scheduler)
# ============================================================================

# Import from scheduler module
from zenith.scheduler import (
    job,
    submit,
    status,
    cancel,
    cluster_info,
    SchedulerClient,
    JobConfig,
    Job as SchedulerJob,
    JobState,
    ClusterStatus,
    set_scheduler_url,
)


# ============================================================================
# Lazy imports for framework-specific adapters
# ============================================================================

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


# ============================================================================
# Public API
# ============================================================================

__all__ = [
    # Core
    "Engine",
    "DataLoader",
    "ZenithBatch",
    
    # Device utilities
    "auto_device",
    "cuda_available",
    
    # Convenience functions
    "load",
    "info",
    
    # Job scheduling
    "job",
    "submit", 
    "status",
    "cancel",
    "cluster_info",
    "SchedulerClient",
    "JobConfig",
    "SchedulerJob",
    "JobState",
    "ClusterStatus",
    "set_scheduler_url",
    
    # Metadata
    "__version__",
    "native_available",
]

