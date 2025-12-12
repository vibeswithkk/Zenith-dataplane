//! Build script for zenith-runtime-cpu
//!
//! This script handles building the C++ NUMA backend when the `numa_cpp` feature is enabled.

fn main() {
    // Only build C++ backend if numa_cpp feature is enabled
    #[cfg(feature = "numa_cpp")]
    build_numa_backend();
    
    // Re-run if these files change
    println!("cargo:rerun-if-changed=../ffi-bindings/cpp/numa_backend.cpp");
    println!("cargo:rerun-if-changed=../ffi-bindings/zenith_numa.h");
    println!("cargo:rerun-if-env-changed=ZENITH_NUMA_CPP");
}

#[cfg(feature = "numa_cpp")]
fn build_numa_backend() {
    use std::env;
    use std::path::PathBuf;
    
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ffi_dir = PathBuf::from(&manifest_dir).join("../ffi-bindings");
    let cpp_dir = ffi_dir.join("cpp");
    
    // Check if CMake is available and libnuma is installed
    let use_cmake = std::process::Command::new("cmake")
        .arg("--version")
        .output()
        .is_ok();
    
    if use_cmake && cpp_dir.join("CMakeLists.txt").exists() {
        // Use CMake for building
        let dst = cmake::Config::new(&cpp_dir)
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("BUILD_TESTS", "OFF")
            .build();
        
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
        println!("cargo:rustc-link-lib=static=zenith_numa");
    } else {
        // Fallback: Use cc crate for simple compilation
        println!("cargo:warning=CMake not available, using cc crate fallback");
        
        cc::Build::new()
            .cpp(true)
            .file(cpp_dir.join("numa_backend.cpp"))
            .include(&ffi_dir)
            .flag_if_supported("-std=c++17")
            .flag_if_supported("-O3")
            .compile("zenith_numa");
    }
    
    // Link against libnuma and pthread
    println!("cargo:rustc-link-lib=numa");
    println!("cargo:rustc-link-lib=pthread");
}
