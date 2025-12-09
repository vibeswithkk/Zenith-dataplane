pub mod event;
pub mod ring_buffer;
pub mod engine;
pub mod wasm_host;
pub mod error;
pub mod admin_api;
pub mod validation;

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::record_batch::RecordBatch;
use crate::engine::ZenithEngine;
use crate::event::ZenithEvent;

pub use engine::ZenithEngine as Engine;
pub use event::ZenithEvent as Event;

/// FFI Error codes
pub mod ffi_error {
    /// Success
    pub const SUCCESS: i32 = 0;
    /// Null pointer passed
    pub const NULL_POINTER: i32 = -1;
    /// Buffer full
    pub const BUFFER_FULL: i32 = -2;
    /// Panic occurred (caught safely)
    pub const PANIC: i32 = -3;
    /// FFI/Arrow error
    pub const FFI_ERROR: i32 = -4;
    /// Initialization failed
    pub const INIT_FAILED: i32 = -5;
}

/// Initialize the Zenith Engine
/// Returns a raw pointer to the engine instance.
/// Caller is responsible for calling zenith_free.
/// 
/// # Safety
/// - Returns null on error (including panic)
/// - Caller must call zenith_free to release
#[no_mangle]
pub extern "C" fn zenith_init(buffer_size: u32) -> *mut c_void {
    // Catch any panic to prevent UB at FFI boundary
    let result = catch_unwind(|| {
        match ZenithEngine::new(buffer_size as usize) {
            Ok(engine) => {
                engine.start();
                let boxed = Box::new(engine);
                Box::into_raw(boxed) as *mut c_void
            },
            Err(_) => std::ptr::null_mut(),
        }
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            // Panic occurred - log and return null
            eprintln!("[zenith] PANIC in zenith_init - caught safely");
            std::ptr::null_mut()
        }
    }
}

/// Free the Zenith Engine
/// 
/// # Safety
/// - engine_ptr must be a valid pointer from zenith_init or null
/// - Must not be called twice with the same pointer
#[no_mangle]
pub unsafe extern "C" fn zenith_free(engine_ptr: *mut c_void) {
    if engine_ptr.is_null() {
        return;
    }
    
    let result = catch_unwind(AssertUnwindSafe(|| {
        let engine = Box::from_raw(engine_ptr as *mut ZenithEngine);
        engine.shutdown();
        // Drop handled by Box
    }));
    
    if result.is_err() {
        eprintln!("[zenith] PANIC in zenith_free - caught safely");
    }
}

/// Publish an Arrow RecordBatch via C Data Interface
/// Takes ownership of the FFI structs (they are moved into Rust)
/// 
/// # Returns
/// - 0: Success
/// - -1: Null pointer
/// - -2: Buffer full
/// - -3: Panic occurred
/// - -4: FFI/Arrow error
/// 
/// # Safety
/// - All pointers must be valid
/// - array_ptr and schema_ptr ownership is transferred to Rust
#[no_mangle]
pub unsafe extern "C" fn zenith_publish(
    engine_ptr: *mut c_void,
    array_ptr: *mut FFI_ArrowArray,
    schema_ptr: *mut FFI_ArrowSchema,
    source_id: u32,
    seq_no: u64
) -> i32 {
    // Validate pointers first (outside catch_unwind for clarity)
    if engine_ptr.is_null() || array_ptr.is_null() || schema_ptr.is_null() {
        return ffi_error::NULL_POINTER;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let engine = &*(engine_ptr as *mut ZenithEngine);
        
        // SAFETY: Caller has prepared valid FFI structs
        let array = std::ptr::read(array_ptr);
        let schema = std::ptr::read(schema_ptr);

        match arrow::ffi::from_ffi(array, &schema) {
            Ok(array_data) => {
                let struct_array = arrow::array::StructArray::from(array_data);
                let batch = RecordBatch::from(&struct_array);
                let event = ZenithEvent::new(source_id, seq_no, batch);
                 
                match engine.get_ring_buffer().push(event) {
                    Ok(_) => ffi_error::SUCCESS,
                    Err(_) => ffi_error::BUFFER_FULL,
                }
            },
            Err(_) => ffi_error::FFI_ERROR,
        }
    }));
    
    match result {
        Ok(code) => code,
        Err(_) => {
            eprintln!("[zenith] PANIC in zenith_publish - caught safely");
            ffi_error::PANIC
        }
    }
}

/// Load a WASM plugin
/// 
/// # Returns
/// - 0: Success
/// - -1: Null pointer
/// - -2: Load failed
/// - -3: Panic occurred
/// 
/// # Safety
/// - engine_ptr must be valid pointer from zenith_init
/// - wasm_bytes must point to valid memory of len bytes
#[no_mangle]
pub unsafe extern "C" fn zenith_load_plugin(
    engine_ptr: *mut c_void,
    wasm_bytes: *const u8,
    len: usize
) -> i32 {
    if engine_ptr.is_null() || wasm_bytes.is_null() {
        return ffi_error::NULL_POINTER;
    }
    
    let result = catch_unwind(AssertUnwindSafe(|| {
        let engine = &*(engine_ptr as *mut ZenithEngine);
        let slice = std::slice::from_raw_parts(wasm_bytes, len);
        
        match engine.load_plugin(slice) {
            Ok(_) => ffi_error::SUCCESS,
            Err(_) => ffi_error::BUFFER_FULL, // Reusing -2 for load failed
        }
    }));
    
    match result {
        Ok(code) => code,
        Err(_) => {
            eprintln!("[zenith] PANIC in zenith_load_plugin - caught safely");
            ffi_error::PANIC
        }
    }
}

