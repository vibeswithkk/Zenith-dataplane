# Plugin Development Guide

Learn how to create custom preprocessing plugins for Zenith AI using WebAssembly (WASM).

## Overview

Zenith plugins are compiled to WebAssembly, enabling:
- **Near-native performance** without Python GIL limitations
- **Sandboxed execution** for security
- **Cross-platform compatibility** (Linux, macOS, Windows)

## Prerequisites

- Rust 1.75+ with `wasm32-wasip1` target
- Basic understanding of memory management

```bash
# Install WASM target
rustup target add wasm32-wasip1
```

## Quick Start

### 1. Create Plugin Project

```bash
cargo new --lib my_plugin
cd my_plugin
```

### 2. Configure Cargo.toml

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# Keep dependencies minimal for small WASM size
```

### 3. Write Plugin Code

```rust
// src/lib.rs

#![no_std]
extern crate alloc;

/// Plugin entry point - called by Zenith runtime
#[no_mangle]
pub extern "C" fn process(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_len: *mut usize,
) -> i32 {
    // Your preprocessing logic here
    
    0 // Return 0 for success, non-zero for error
}

/// Plugin metadata
#[no_mangle]
pub extern "C" fn plugin_info() -> *const u8 {
    static INFO: &[u8] = b"my-plugin v0.1.0\0";
    INFO.as_ptr()
}
```

### 4. Build

```bash
cargo build --target wasm32-wasip1 --release
```

Output: `target/wasm32-wasip1/release/my_plugin.wasm`

### 5. Use in Python

```python
import zenith

engine = zenith.Engine()
engine.load_plugin("path/to/my_plugin.wasm")
```

## Plugin Interface

### Required Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `process` | `(ptr, len, out_ptr, out_len) -> i32` | Main processing entry point |
| `plugin_info` | `() -> *const u8` | Return plugin name and version |

### Optional Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `plugin_version` | `() -> u32` | Numeric version for compatibility checks |
| `init` | `() -> i32` | Called once when plugin is loaded |
| `cleanup` | `() -> ()` | Called when plugin is unloaded |

## Memory Management

### Reading Input Data

```rust
#[no_mangle]
pub extern "C" fn process(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_len: *mut usize,
) -> i32 {
    // Safety: Trust host-provided pointers
    let input = unsafe {
        core::slice::from_raw_parts(input_ptr, input_len)
    };
    
    // Process input...
    
    0
}
```

### Writing Output Data

```rust
#[no_mangle]
pub extern "C" fn process(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_len: *mut usize,
) -> i32 {
    let result: Vec<u8> = process_data(input);
    
    // Write to output buffer
    unsafe {
        core::ptr::copy_nonoverlapping(
            result.as_ptr(),
            output_ptr,
            result.len()
        );
        *output_len = result.len();
    }
    
    0
}
```

## Example: Image Resize Plugin

```rust
#![no_std]
extern crate alloc;

use alloc::vec::Vec;

#[repr(C)]
pub struct ResizeParams {
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
}

#[no_mangle]
pub extern "C" fn resize_image(
    input_ptr: *const u8,
    params_ptr: *const ResizeParams,
    output_ptr: *mut u8,
) -> i32 {
    let params = unsafe { &*params_ptr };
    let input = unsafe {
        core::slice::from_raw_parts(
            input_ptr,
            (params.src_width * params.src_height * 3) as usize
        )
    };
    
    // Nearest-neighbor resize
    let x_ratio = params.src_width as f32 / params.dst_width as f32;
    let y_ratio = params.src_height as f32 / params.dst_height as f32;
    
    for y in 0..params.dst_height {
        for x in 0..params.dst_width {
            let src_x = (x as f32 * x_ratio) as u32;
            let src_y = (y as f32 * y_ratio) as u32;
            
            let src_idx = ((src_y * params.src_width + src_x) * 3) as usize;
            let dst_idx = ((y * params.dst_width + x) * 3) as usize;
            
            unsafe {
                *output_ptr.add(dst_idx) = input[src_idx];
                *output_ptr.add(dst_idx + 1) = input[src_idx + 1];
                *output_ptr.add(dst_idx + 2) = input[src_idx + 2];
            }
        }
    }
    
    0
}
```

## Example: Text Tokenizer Plugin

```rust
#![no_std]
extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;

// Simple whitespace tokenizer
#[no_mangle]
pub extern "C" fn tokenize(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u32,
    output_len: *mut usize,
) -> i32 {
    let input = unsafe {
        core::slice::from_raw_parts(input_ptr, input_len)
    };
    
    let text = match core::str::from_utf8(input) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    let mut token_count = 0;
    for word in text.split_whitespace() {
        // Simple hash as token ID (replace with real vocab lookup)
        let token_id = simple_hash(word);
        unsafe {
            *output_ptr.add(token_count) = token_id;
        }
        token_count += 1;
    }
    
    unsafe {
        *output_len = token_count;
    }
    
    0
}

fn simple_hash(s: &str) -> u32 {
    let mut hash: u32 = 0;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    hash
}
```

## Best Practices

### 1. Keep WASM Size Small

```toml
# Cargo.toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
strip = true        # Strip symbols
```

### 2. Handle Errors Gracefully

```rust
#[no_mangle]
pub extern "C" fn process(...) -> i32 {
    match do_work() {
        Ok(_) => 0,
        Err(ErrorKind::InvalidInput) => -1,
        Err(ErrorKind::OutOfMemory) => -2,
        Err(_) => -99,
    }
}
```

### 3. Avoid Allocations in Hot Paths

```rust
// Bad: Allocates on every call
fn process(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();  // Allocation!
    // ...
}

// Good: Reuse buffers
fn process(data: &[u8], output: &mut [u8]) -> usize {
    // Write directly to pre-allocated buffer
}
```

### 4. Document Your Plugin

```rust
//! My Awesome Plugin
//!
//! This plugin does XYZ preprocessing at blazing speed.
//!
//! ## Functions
//! - `process`: Main entry point
//! - `configure`: Set preprocessing options
```

## Testing Plugins

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resize() {
        let input = vec![255u8; 100 * 100 * 3]; // 100x100 RGB
        let mut output = vec![0u8; 50 * 50 * 3]; // 50x50 RGB
        
        let params = ResizeParams {
            src_width: 100,
            src_height: 100,
            dst_width: 50,
            dst_height: 50,
        };
        
        let result = resize_image(
            input.as_ptr(),
            &params,
            output.as_mut_ptr()
        );
        
        assert_eq!(result, 0);
        assert!(output.iter().all(|&x| x == 255));
    }
}
```

Run tests:
```bash
cargo test
```

## Debugging

### Print Debugging (via Host)

```rust
// Declare external log function
extern "C" {
    fn host_log(ptr: *const u8, len: usize);
}

fn log(msg: &str) {
    unsafe {
        host_log(msg.as_ptr(), msg.len());
    }
}
```

### WASM Inspection

```bash
# View exports
wasm-objdump -x my_plugin.wasm

# Disassemble
wasm-objdump -d my_plugin.wasm
```

## Publishing Your Plugin

1. Build release version:
   ```bash
   cargo build --target wasm32-wasip1 --release
   ```

2. Add to your project's `plugins/` directory

3. Document usage in README

4. Consider publishing to crates.io for Rust users
