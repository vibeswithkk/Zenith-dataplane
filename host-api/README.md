# Zenith Host API

The Host API defines functions that WASM plugins can call to interact with the Zenith runtime environment.

## Overview

Plugins run in a sandboxed WASM environment but often need to:
- Log messages
- Get current time
- Access event metadata
- Generate random numbers

The Host API provides these capabilities in a **safe, controlled manner**.

## Available Functions

### Logging

```rust
extern "C" {
    fn zenith_host_log(level: u32, message_ptr: *const u8, message_len: usize) -> i32;
}
```

**Log Levels:**
- `0` = Debug
- `1` = Info
- `2` = Warn
- `3` = Error

**Example:**
```rust
let msg = "Processing event";
unsafe {
    zenith_host_log(1, msg.as_ptr(), msg.len());
}
```

### Timestamp Access

```rust
extern "C" {
    fn zenith_host_get_timestamp_ns() -> u64;
}
```

Returns nanoseconds since UNIX epoch.

### Random Numbers

```rust
extern "C" {
    fn zenith_host_get_random_u64() -> u64;
}
```

Provides non-cryptographic randomness for sampling/jittering.

### Event Field Access

```rust
extern "C" {
    fn zenith_host_read_event_field(
        field_index: u32,
        out_buffer: *mut u8,
        out_buffer_len: usize
    ) -> i32;
}
```

Returns `-1` on error, or number of bytes written.

## Security Model

1. **Capability-Based**: Plugins only have access to explicitly provided functions
2. **No Direct I/O**: Plugins cannot access filesystem, network, or system resources
3. **Metered**: All host calls are counted and can be rate-limited
4. **Sandboxed**: Runs in WASM sandbox with memory isolation

## Building Plugins with Host API

See `examples/plugin_with_host_api.rs` for a complete example.

Compile to WASM:
```bash
rustc --target wasm32-wasi examples/plugin_with_host_api.rs --crate-type cdylib -o plugin.wasm
```

## Testing

```bash
cargo test
```

All host functions include unit tests.

## Integration

The host API is automatically linked into the WASM runtime when plugins are loaded by the Zenith engine. No manual linking required.
