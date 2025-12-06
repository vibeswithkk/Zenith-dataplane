"""
Python FFI binding test
"""
import ctypes
import sys
import os

# Load the shared library
lib_path = os.path.join(os.path.dirname(__file__), "../../core/target/release/libzenith_core.so")

try:
    lib = ctypes.CDLL(lib_path)
    
    # Define function signatures
    lib.zenith_init.argtypes = [ctypes.c_uint32]
    lib.zenith_init.restype = ctypes.c_void_p
    
    lib.zenith_free.argtypes = [ctypes.c_void_p]
    lib.zenith_free.restype = None
    
    # Initialize
    engine = lib.zenith_init(1024)
    if not engine:
        print("❌ Failed to initialize engine")
        sys.exit(1)
    
    print("✅ Zenith engine initialized successfully")
    print(f"Engine handle: {hex(engine)}")
    
    # Cleanup
    lib.zenith_free(engine)
    print("✅ Engine freed successfully")
    
except Exception as e:
    print(f"❌ Error: {e}")
    sys.exit(1)
