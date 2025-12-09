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

__version__ = "0.1.0"
__author__ = "Zenith Contributors"

# ============================================================================
# Core imports
# ============================================================================

# Try to import native Rust extension first
_NATIVE_AVAILABLE = False
try:
    from zenith._core import Engine as NativeEngine
    from zenith._core import version as native_version
    from zenith._core import is_available
    _NATIVE_AVAILABLE = is_available()
except ImportError:
    _NATIVE_AVAILABLE = False

# Import Python implementations
from zenith.engine import Engine as PythonEngine
from zenith.loader import DataLoader

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
# Job scheduling (SLURM alternative) - coming soon
# ============================================================================

def job(gpus=1, memory="8GB", timeout="1h", **kwargs):
    """
    Decorator to mark a function as a Zenith job.
    
    This is a simpler alternative to SLURM for ML workloads.
    
    Args:
        gpus: Number of GPUs required
        memory: Memory requirement (e.g., "16GB")
        timeout: Maximum runtime (e.g., "2h", "1d")
        **kwargs: Additional job options
        
    Example:
        >>> @zenith.job(gpus=4, memory="32GB")
        ... def train_model():
        ...     model = MyModel()
        ...     model.fit(data)
        
        >>> zenith.submit(train_model)
    """
    def decorator(func):
        func._zenith_job = {
            "gpus": gpus,
            "memory": memory,
            "timeout": timeout,
            **kwargs
        }
        return func
    return decorator


def submit(func, *args, **kwargs):
    """
    Submit a job for execution.
    
    Args:
        func: A function decorated with @zenith.job
        *args: Arguments to pass to the function
        **kwargs: Keyword arguments to pass
        
    Returns:
        Job ID for tracking
        
    Example:
        >>> job_id = zenith.submit(train_model, epochs=100)
        >>> zenith.status(job_id)
    """
    if not hasattr(func, '_zenith_job'):
        raise ValueError(
            f"Function {func.__name__} is not a Zenith job. "
            "Decorate it with @zenith.job() first."
        )
    
    # TODO: Implement actual job submission
    job_config = func._zenith_job
    print(f"[zenith] Submitting job: {func.__name__}")
    print(f"[zenith] Config: GPUs={job_config['gpus']}, Memory={job_config['memory']}")
    
    # For now, just run locally
    return func(*args, **kwargs)


def status(job_id=None):
    """
    Check status of running jobs.
    
    Args:
        job_id: Specific job to check (None = all jobs)
    """
    # TODO: Implement job status checking
    print("[zenith] No jobs running (scheduler not yet implemented)")


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
    
    # Convenience functions
    "load",
    "info",
    
    # Job scheduling
    "job",
    "submit", 
    "status",
    
    # Metadata
    "__version__",
    "native_available",
]
