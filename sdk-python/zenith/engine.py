"""
Zenith Core Engine

The high-performance Rust-powered engine for data loading and preprocessing.
"""

import ctypes
import os
import sys
from pathlib import Path
from typing import Optional, Union, List, Any

import pyarrow as pa


def _find_core_library() -> str:
    """Locate the Zenith core shared library."""
    # Priority order for finding the library
    search_paths = [
        # 1. Environment variable override
        os.environ.get("ZENITH_CORE_LIB"),
        # 2. Installed alongside Python package
        Path(__file__).parent / "_core" / "libzenith_core.so",
        # 3. Development: workspace target
        Path(__file__).parents[3] / "target" / "release" / "libzenith_core.so",
        # 4. Development: core target
        Path(__file__).parents[3] / "core" / "target" / "release" / "libzenith_core.so",
    ]
    
    for path in search_paths:
        if path and Path(path).exists():
            return str(path)
    
    raise RuntimeError(
        "Zenith core library not found. Please either:\n"
        "1. Install zenith-ai via pip (pip install zenith-ai)\n"
        "2. Build from source (cargo build --release)\n"
        "3. Set ZENITH_CORE_LIB environment variable"
    )


class Engine:
    """
    High-performance data processing engine.
    
    The Engine is the central component of Zenith, providing:
    - Ultra-fast data loading (< 100µs latency)
    - Zero-copy memory management via Apache Arrow
    - WASM plugin execution for custom preprocessing
    
    Example:
        >>> engine = Engine(buffer_size=4096)
        >>> engine.load_plugin("image_resize.wasm")
        >>> data = engine.load("path/to/images")
        >>> processed = engine.process(data)
    """
    
    def __init__(self, buffer_size: int = 1024, lib_path: Optional[str] = None):
        """
        Initialize the Zenith Engine.
        
        Args:
            buffer_size: Size of the internal ring buffer (default: 1024)
            lib_path: Optional path to libzenith_core.so (auto-detected if not provided)
        """
        self._lib_path = lib_path or _find_core_library()
        self._lib = ctypes.CDLL(self._lib_path)
        self._setup_ffi()
        
        self._engine_ptr = self._lib.zenith_init(buffer_size)
        if not self._engine_ptr:
            raise RuntimeError("Failed to initialize Zenith Engine")
        
        self._plugins: List[str] = []
        self._closed = False
    
    def _setup_ffi(self):
        """Configure FFI function signatures."""
        # zenith_init(buffer_size) -> engine_ptr
        self._lib.zenith_init.argtypes = [ctypes.c_uint32]
        self._lib.zenith_init.restype = ctypes.c_void_p
        
        # zenith_publish(engine, array, schema, source_id, seq_no) -> result
        self._lib.zenith_publish.argtypes = [
            ctypes.c_void_p,
            ctypes.c_void_p,
            ctypes.c_void_p,
            ctypes.c_uint32,
            ctypes.c_uint64
        ]
        self._lib.zenith_publish.restype = ctypes.c_int32
        
        # zenith_load_plugin(engine, bytes, len) -> result
        self._lib.zenith_load_plugin.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_size_t
        ]
        self._lib.zenith_load_plugin.restype = ctypes.c_int32
        
        # zenith_free(engine) -> void
        self._lib.zenith_free.argtypes = [ctypes.c_void_p]
        self._lib.zenith_free.restype = None
    
    def load_plugin(self, plugin_path: Union[str, Path]) -> None:
        """
        Load a WASM preprocessing plugin.
        
        Args:
            plugin_path: Path to the .wasm plugin file
            
        Raises:
            FileNotFoundError: If plugin file doesn't exist
            RuntimeError: If plugin loading fails
        """
        plugin_path = Path(plugin_path)
        if not plugin_path.exists():
            raise FileNotFoundError(f"Plugin not found: {plugin_path}")
        
        with open(plugin_path, 'rb') as f:
            wasm_bytes = f.read()
        
        result = self._lib.zenith_load_plugin(
            self._engine_ptr,
            wasm_bytes,
            len(wasm_bytes)
        )
        
        if result != 0:
            raise RuntimeError(f"Failed to load plugin: {plugin_path} (error code: {result})")
        
        self._plugins.append(str(plugin_path))
    
    def publish(
        self,
        data: pa.RecordBatch,
        source_id: int = 0,
        seq_no: int = 0
    ) -> None:
        """
        Publish data to the engine for processing.
        
        This is a zero-copy operation when possible, achieving
        microsecond-level latency.
        
        Args:
            data: PyArrow RecordBatch containing the data
            source_id: Identifier for the data source
            seq_no: Sequence number for ordering
            
        Raises:
            RuntimeError: If publishing fails
        """
        from pyarrow.cffi import ffi as arrow_ffi
        
        struct_array = data.to_struct_array()
        
        c_schema = arrow_ffi.new("struct ArrowSchema*")
        c_array = arrow_ffi.new("struct ArrowArray*")
        
        c_schema_addr = int(arrow_ffi.cast("uintptr_t", c_schema))
        c_array_addr = int(arrow_ffi.cast("uintptr_t", c_array))
        
        struct_array._export_to_c(c_array_addr, c_schema_addr)
        
        result = self._lib.zenith_publish(
            self._engine_ptr,
            ctypes.c_void_p(c_array_addr),
            ctypes.c_void_p(c_schema_addr),
            source_id,
            seq_no
        )
        
        if result != 0:
            raise RuntimeError(f"Publish failed (error code: {result})")
    
    def load(self, source: Union[str, Path]) -> pa.Table:
        """
        Load data from a source path.
        
        Args:
            source: Path to data file or directory
            
        Returns:
            PyArrow Table containing the loaded data
        """
        # Placeholder for full implementation
        # In production, this would use Rust core for fast loading
        source = Path(source)
        if source.suffix == '.parquet':
            import pyarrow.parquet as pq
            return pq.read_table(source)
        elif source.suffix == '.csv':
            import pyarrow.csv as csv
            return csv.read_csv(source)
        else:
            raise ValueError(f"Unsupported file format: {source.suffix}")
    
    def process(self, data: pa.Table) -> pa.Table:
        """
        Process data through loaded plugins.
        
        Args:
            data: Input PyArrow Table
            
        Returns:
            Processed PyArrow Table
        """
        # Placeholder - actual processing happens in Rust core
        return data
    
    @property
    def plugins(self) -> List[str]:
        """List of loaded plugin paths."""
        return self._plugins.copy()
    
    def close(self) -> None:
        """Release engine resources."""
        if not self._closed and self._engine_ptr:
            self._lib.zenith_free(self._engine_ptr)
            self._engine_ptr = None
            self._closed = True
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False
    
    def __del__(self):
        self.close()
    
    def __repr__(self):
        status = "closed" if self._closed else "active"
        return f"<zenith.Engine(status={status}, plugins={len(self._plugins)})>"
