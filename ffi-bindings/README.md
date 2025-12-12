# FFI Bindings for Zenith Core

This directory contains the Foreign Function Interface (FFI) definitions for the Zenith Data Plane core engine.

## Files

### `zenith_core.h`
C header file defining the public API for interacting with Zenith from any language that supports C FFI (Go, Python, Node.js, Ruby, etc.).

### `zenith_numa.h`
C header file for the NUMA (Non-Uniform Memory Access) backend providing:
- NUMA-aware memory allocation
- Thread binding to NUMA nodes/CPUs
- Topology discovery
- Memory policies (interleave, membind)

## C++ Backends

The `cpp/` directory contains high-performance C++ implementations:

### NUMA Backend (`cpp/numa_backend.cpp`)
Native libnuma integration for optimal memory locality in multi-socket systems.

**Build:**
```bash
cd cpp && mkdir build && cd build
cmake .. && make
```

**Dependencies:**
- libnuma-dev (Ubuntu/Debian: `apt install libnuma-dev`)
- cmake 3.16+
- C++17 compiler

## API Overview

### Core Engine
```c
ZenithEngine engine = zenith_init(1024);
zenith_publish(engine, array_ptr, schema_ptr, source_id, seq_no);
zenith_load_plugin(engine, wasm_bytes, wasm_len);
zenith_free(engine);
```

### NUMA Backend
```c
zenith_numa_init();
void* ptr = zenith_numa_alloc_onnode(4096, 0);  // Alloc on node 0
zenith_numa_bind_thread_to_node(0);             // Bind thread
zenith_numa_free(ptr, 4096);
zenith_numa_cleanup();
```

## Language Bindings

The header files are used by:
- **Go**: Via CGO (`import "C"`)
- **Python**: Via ctypes or cffi
- **Node.js**: Via ffi-napi
- **Rust**: Via zenith-runtime-cpu with `numa_cpp` feature

## Error Codes

### Core Engine
| Code | Meaning              |
|------|----------------------|
| 0    | Success              |
| -1   | Null pointer error   |
| -2   | Buffer full          |
| -3   | Plugin load error    |
| -4   | FFI conversion error |

### NUMA Backend
| Code | Meaning              |
|------|----------------------|
| 0    | Success              |
| -1   | NUMA unavailable     |
| -2   | Invalid node         |
| -3   | Allocation failed    |
| -4   | Bind failed          |
| -5   | Null pointer         |

## Rust Integration

Enable the C++ NUMA backend in Rust:
```bash
cargo build -p zenith-runtime-cpu --features numa_cpp
```

## Testing

```bash
# C++ tests (requires GTest)
cd cpp/build && cmake .. -DBUILD_TESTS=ON && make && ./numa_test

# Rust tests
cargo test -p zenith-runtime-cpu --features numa_cpp
```

