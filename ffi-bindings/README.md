# FFI Bindings for Zenith Core

This directory contains the Foreign Function Interface (FFI) definitions for the Zenith Data Plane core engine.

## Files

### `zenith_core.h`
C header file defining the public API for interacting with Zenith from any language that supports C FFI (Go, Python, Node.js, Ruby, etc.).

## API Overview

### Initialization
```c
ZenithEngine engine = zenith_init(1024);  // Create engine with buffer size
```

### Publishing Events
```c
int32_t result = zenith_publish(engine, array_ptr, schema_ptr, source_id, seq_no);
```

### Loading Plugins
```c
int32_t result = zenith_load_plugin(engine, wasm_bytes, wasm_len);
```

### Cleanup
```c
zenith_free(engine);
```

## Language Bindings

The header file is used by:
- **Go**: Via CGO (`import "C"`)
- **Python**: Via ctypes or cffi
- **Node.js**: Via ffi-napi
- **Rust**: Via bindgen (for external crates)

## Error Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| -1 | Null pointer error |
| -2 | Buffer full |
| -3 | Plugin load error |
| -4 | FFI conversion error |

## Build Integration

This header is automatically available when building the core library:

```bash
cd core
cargo build --release
# Header is reference-only, symbols exported via lib
```

## Generating Bindings

Use `cbindgen` to keep synchronized with Rust source:

```bash
cbindgen --config cbindgen.toml --crate zenith-core --output ffi-bindings/zenith_core.h
```

## Testing

See `examples/` directory for FFI usage examples in various languages.
