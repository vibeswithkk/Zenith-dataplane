# Zenith C++ Backends - CMake Configuration
#
# Builds native C++ performance backends for NUMA and ONNX Runtime.
#
# Usage:
#   mkdir build && cd build
#   cmake .. && make
#
# To install libnuma on Ubuntu/Debian:
#   sudo apt-get install libnuma-dev

cmake_minimum_required(VERSION 3.16)
project(zenith_cpp_backends VERSION 1.0.0 LANGUAGES CXX)

# C++ Standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Position Independent Code (required for shared libraries)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Compiler flags
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    add_compile_options(-Wall -Wextra -Wpedantic)
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        add_compile_options(-O3 -march=native)
    endif()
endif()

# Include parent directory for header files
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/..)

# ============================================================================
# NUMA Backend
# ============================================================================

option(BUILD_NUMA_BACKEND "Build NUMA backend (requires libnuma)" ON)

if(BUILD_NUMA_BACKEND)
    # Try to find libnuma
    find_package(PkgConfig)
    if(PkgConfig_FOUND)
        pkg_check_modules(NUMA numa)
    endif()
    
    # Check if numa.h exists
    include(CheckIncludeFileCXX)
    check_include_file_cxx("numa.h" HAVE_NUMA_H)
    
    if(NUMA_FOUND OR HAVE_NUMA_H)
        message(STATUS "Building NUMA backend with libnuma support")
        
        add_library(zenith_numa STATIC
            numa_backend.cpp
        )
        
        target_include_directories(zenith_numa PUBLIC
            ${CMAKE_CURRENT_SOURCE_DIR}/..
            ${NUMA_INCLUDE_DIRS}
        )
        
        target_link_libraries(zenith_numa PUBLIC
            ${NUMA_LIBRARIES}
            numa
            pthread
        )
        
        # Define ZENITH_HAS_LIBNUMA to enable libnuma code path
        target_compile_definitions(zenith_numa PRIVATE
            ZENITH_HAS_LIBNUMA=1
        )
        
    else()
        message(STATUS "Building NUMA backend without libnuma (stub implementation)")
        
        add_library(zenith_numa STATIC
            numa_backend.cpp
        )
        
        target_include_directories(zenith_numa PUBLIC
            ${CMAKE_CURRENT_SOURCE_DIR}/..
        )
    endif()
    
    # Install target
    install(TARGETS zenith_numa
        ARCHIVE DESTINATION lib
        LIBRARY DESTINATION lib
    )
    
    # Install headers
    install(FILES
        ${CMAKE_CURRENT_SOURCE_DIR}/../zenith_numa.h
        DESTINATION include
    )
endif()

# ============================================================================
# ONNX Backend (Phase 2 - Future)
# ============================================================================

option(BUILD_ONNX_BACKEND "Build ONNX Runtime backend" OFF)

if(BUILD_ONNX_BACKEND)
    message(STATUS "ONNX backend is planned for Phase 2")
    # Future: Add ONNX Runtime integration
endif()

# ============================================================================
# Tests (Optional)
# ============================================================================

option(BUILD_TESTS "Build tests" OFF)

if(BUILD_TESTS)
    enable_testing()
    
    # Find or fetch Google Test
    find_package(GTest QUIET)
    if(NOT GTest_FOUND)
        include(FetchContent)
        FetchContent_Declare(
            googletest
            URL https://github.com/google/googletest/archive/refs/tags/v1.14.0.zip
        )
        FetchContent_MakeAvailable(googletest)
    endif()
    
    # NUMA tests
    if(BUILD_NUMA_BACKEND)
        add_executable(numa_test
            tests/numa_test.cpp
        )
        target_link_libraries(numa_test
            zenith_numa
            GTest::gtest_main
        )
        add_test(NAME numa_test COMMAND numa_test)
    endif()
endif()

# ============================================================================
# Summary
# ============================================================================

message(STATUS "")
message(STATUS "Zenith C++ Backends Configuration:")
message(STATUS "  NUMA Backend: ${BUILD_NUMA_BACKEND}")
message(STATUS "  ONNX Backend: ${BUILD_ONNX_BACKEND}")
message(STATUS "  Tests:        ${BUILD_TESTS}")
message(STATUS "")
