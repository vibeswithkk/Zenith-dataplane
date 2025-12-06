package main

/*
#cgo LDFLAGS: -L../../core/target/release -lzenith_core
#include "../../ffi-bindings/zenith_core.h"
*/
import "C"
import (
	"fmt"
)

func main() {
	// Initialize engine
	engine := C.zenith_init(1024)
	if engine == nil {
		panic("Failed to initialize Zenith engine")
	}
	defer C.zenith_free(engine)

	fmt.Println("✅ Zenith engine initialized successfully")

	// Example: Load a plugin
	// In real usage, you'd read the WASM file
	// wasmBytes := readFile("plugin.wasm")
	// ret := C.zenith_load_plugin(engine, (*C.uint8_t)(unsafe.Pointer(&wasmBytes[0])), C.size_t(len(wasmBytes)))

	fmt.Println("Go FFI binding test completed")
}
