/**
 * Zenith NUMA Backend - C++ Implementation
 *
 * Native NUMA memory operations using libnuma for high-performance
 * ML data loading with optimal memory locality.
 *
 * Copyright 2025 Zenith Contributors
 * SPDX-License-Identifier: Apache-2.0
 *
 * Build with libnuma:
 *   g++ -DZENITH_HAS_LIBNUMA -lnuma numa_backend.cpp
 *
 * Build without libnuma (fallback):
 *   g++ numa_backend.cpp
 */

#include "../zenith_numa.h"
#include <cstdlib>
#include <cstring>

// Check if libnuma is available - define ZENITH_HAS_LIBNUMA when building with
// libnuma CMake will define this automatically when libnuma is found
#if defined(ZENITH_HAS_LIBNUMA) && defined(__linux__)
#define ZENITH_USE_LIBNUMA 1
#include <numa.h>
#include <numaif.h>
#include <pthread.h>
#include <sched.h>
#else
#define ZENITH_USE_LIBNUMA 0
#endif

// Track initialization state
static bool g_numa_initialized = false;

#if ZENITH_USE_LIBNUMA

extern "C" {

/* ============================================================================
 * Initialization and Cleanup
 * ============================================================================
 */

int32_t zenith_numa_init(void) {
  if (g_numa_initialized) {
    return ZENITH_NUMA_OK;
  }

  if (numa_available() < 0) {
    return ZENITH_NUMA_ERR_UNAVAILABLE;
  }

  g_numa_initialized = true;
  return ZENITH_NUMA_OK;
}

void zenith_numa_cleanup(void) { g_numa_initialized = false; }

int32_t zenith_numa_available(void) { return numa_available() >= 0 ? 1 : 0; }

/* ============================================================================
 * Topology Queries
 * ============================================================================
 */

int32_t zenith_numa_num_nodes(void) {
  if (!g_numa_initialized) {
    return 0;
  }
  return numa_num_configured_nodes();
}

int32_t zenith_numa_num_cpus(void) {
  if (!g_numa_initialized) {
    return 0;
  }
  return numa_num_configured_cpus();
}

int32_t zenith_numa_node_of_cpu(int32_t cpu) {
  if (!g_numa_initialized || cpu < 0) {
    return -1;
  }
  return numa_node_of_cpu(cpu);
}

int32_t zenith_numa_preferred_node(void) {
  if (!g_numa_initialized) {
    return 0;
  }
  return numa_preferred();
}

/* ============================================================================
 * Memory Allocation
 * ============================================================================
 */

void *zenith_numa_alloc_onnode(size_t size, int32_t node) {
  if (!g_numa_initialized) {
    return nullptr;
  }

  if (node < 0 || node >= numa_num_configured_nodes()) {
    return nullptr;
  }

  void *ptr = numa_alloc_onnode(size, node);
  if (ptr != nullptr) {
    // Touch the memory to ensure it's actually allocated on the node
    // (first-touch policy)
    memset(ptr, 0, size);
  }

  return ptr;
}

void *zenith_numa_alloc_interleaved(size_t size) {
  if (!g_numa_initialized) {
    return nullptr;
  }

  void *ptr = numa_alloc_interleaved(size);
  if (ptr != nullptr) {
    memset(ptr, 0, size);
  }

  return ptr;
}

void *zenith_numa_alloc_local(size_t size) {
  if (!g_numa_initialized) {
    return nullptr;
  }

  void *ptr = numa_alloc_local(size);
  if (ptr != nullptr) {
    memset(ptr, 0, size);
  }

  return ptr;
}

void zenith_numa_free(void *ptr, size_t size) {
  if (ptr != nullptr && size > 0) {
    numa_free(ptr, size);
  }
}

/* ============================================================================
 * Thread Binding
 * ============================================================================
 */

int32_t zenith_numa_bind_thread_to_node(int32_t node) {
  if (!g_numa_initialized) {
    return ZENITH_NUMA_ERR_UNAVAILABLE;
  }

  if (node < 0 || node >= numa_num_configured_nodes()) {
    return ZENITH_NUMA_ERR_INVALID_NODE;
  }

  struct bitmask *nodemask = numa_allocate_nodemask();
  if (nodemask == nullptr) {
    return ZENITH_NUMA_ERR_ALLOC_FAILED;
  }

  numa_bitmask_setbit(nodemask, node);
  int result = numa_run_on_node_mask(nodemask);
  numa_free_nodemask(nodemask);

  return result == 0 ? ZENITH_NUMA_OK : ZENITH_NUMA_ERR_BIND_FAILED;
}

int32_t zenith_numa_bind_thread_to_cpu(int32_t cpu) {
  if (!g_numa_initialized) {
    return ZENITH_NUMA_ERR_UNAVAILABLE;
  }

  if (cpu < 0 || cpu >= numa_num_configured_cpus()) {
    return ZENITH_NUMA_ERR_INVALID_NODE;
  }

  cpu_set_t cpuset;
  CPU_ZERO(&cpuset);
  CPU_SET(cpu, &cpuset);

  int result = pthread_setaffinity_np(pthread_self(), sizeof(cpuset), &cpuset);
  return result == 0 ? ZENITH_NUMA_OK : ZENITH_NUMA_ERR_BIND_FAILED;
}

int32_t zenith_numa_unbind_thread(void) {
  if (!g_numa_initialized) {
    return ZENITH_NUMA_ERR_UNAVAILABLE;
  }

  // Reset to all nodes
  struct bitmask *all_nodes = numa_allocate_nodemask();
  if (all_nodes == nullptr) {
    return ZENITH_NUMA_ERR_ALLOC_FAILED;
  }

  // Set all bits for all configured nodes
  for (int i = 0; i < numa_num_configured_nodes(); i++) {
    numa_bitmask_setbit(all_nodes, i);
  }

  numa_run_on_node_mask(all_nodes);
  numa_free_nodemask(all_nodes);

  return ZENITH_NUMA_OK;
}

/* ============================================================================
 * Memory Policies
 * ============================================================================
 */

int32_t zenith_numa_set_preferred(int32_t node) {
  if (!g_numa_initialized) {
    return ZENITH_NUMA_ERR_UNAVAILABLE;
  }

  numa_set_preferred(node);
  return ZENITH_NUMA_OK;
}

int32_t zenith_numa_set_interleave(uint64_t nodemask) {
  if (!g_numa_initialized) {
    return ZENITH_NUMA_ERR_UNAVAILABLE;
  }

  struct bitmask *mask = numa_allocate_nodemask();
  if (mask == nullptr) {
    return ZENITH_NUMA_ERR_ALLOC_FAILED;
  }

  int max_nodes = numa_num_configured_nodes();
  for (int i = 0; i < 64 && i < max_nodes; i++) {
    if (nodemask & (1ULL << i)) {
      numa_bitmask_setbit(mask, i);
    }
  }

  numa_set_interleave_mask(mask);
  numa_free_nodemask(mask);

  return ZENITH_NUMA_OK;
}

int32_t zenith_numa_set_membind(uint64_t nodemask) {
  if (!g_numa_initialized) {
    return ZENITH_NUMA_ERR_UNAVAILABLE;
  }

  struct bitmask *mask = numa_allocate_nodemask();
  if (mask == nullptr) {
    return ZENITH_NUMA_ERR_ALLOC_FAILED;
  }

  int max_nodes = numa_num_configured_nodes();
  for (int i = 0; i < 64 && i < max_nodes; i++) {
    if (nodemask & (1ULL << i)) {
      numa_bitmask_setbit(mask, i);
    }
  }

  numa_set_membind(mask);
  numa_free_nodemask(mask);

  return ZENITH_NUMA_OK;
}

/* ============================================================================
 * Statistics and Information
 * ============================================================================
 */

int32_t zenith_numa_get_node_info(int32_t node, ZenithNumaNodeInfo *info) {
  if (!g_numa_initialized) {
    return ZENITH_NUMA_ERR_UNAVAILABLE;
  }

  if (info == nullptr) {
    return ZENITH_NUMA_ERR_NULL_PTR;
  }

  if (node < 0 || node >= numa_num_configured_nodes()) {
    return ZENITH_NUMA_ERR_INVALID_NODE;
  }

  info->node_id = node;

  // Get memory information
  long long free_mem = 0;
  info->total_memory = numa_node_size64(node, &free_mem);
  info->free_memory = static_cast<uint64_t>(free_mem);

  // Count CPUs on this node
  struct bitmask *cpumask = numa_allocate_cpumask();
  if (cpumask != nullptr) {
    numa_node_to_cpus(node, cpumask);
    info->num_cpus = numa_bitmask_weight(cpumask);
    numa_free_cpumask(cpumask);
  } else {
    info->num_cpus = 0;
  }

  return ZENITH_NUMA_OK;
}

int32_t zenith_numa_distance(int32_t node1, int32_t node2) {
  if (!g_numa_initialized) {
    return -1;
  }

  int max_nodes = numa_num_configured_nodes();
  if (node1 < 0 || node1 >= max_nodes || node2 < 0 || node2 >= max_nodes) {
    return -1;
  }

  return numa_distance(node1, node2);
}

} // extern "C"

#else // Fallback stubs (no libnuma available)

extern "C" {

int32_t zenith_numa_init(void) { return ZENITH_NUMA_ERR_UNAVAILABLE; }

void zenith_numa_cleanup(void) {}

int32_t zenith_numa_available(void) { return 0; }

int32_t zenith_numa_num_nodes(void) { return 1; }

int32_t zenith_numa_num_cpus(void) { return 1; }

int32_t zenith_numa_node_of_cpu(int32_t cpu) {
  (void)cpu;
  return 0;
}

int32_t zenith_numa_preferred_node(void) { return 0; }

void *zenith_numa_alloc_onnode(size_t size, int32_t node) {
  (void)node;
  return malloc(size);
}

void *zenith_numa_alloc_interleaved(size_t size) { return malloc(size); }

void *zenith_numa_alloc_local(size_t size) { return malloc(size); }

void zenith_numa_free(void *ptr, size_t size) {
  (void)size;
  free(ptr);
}

int32_t zenith_numa_bind_thread_to_node(int32_t node) {
  (void)node;
  return ZENITH_NUMA_ERR_UNAVAILABLE;
}

int32_t zenith_numa_bind_thread_to_cpu(int32_t cpu) {
  (void)cpu;
  return ZENITH_NUMA_ERR_UNAVAILABLE;
}

int32_t zenith_numa_unbind_thread(void) { return ZENITH_NUMA_OK; }

int32_t zenith_numa_set_preferred(int32_t node) {
  (void)node;
  return ZENITH_NUMA_ERR_UNAVAILABLE;
}

int32_t zenith_numa_set_interleave(uint64_t nodemask) {
  (void)nodemask;
  return ZENITH_NUMA_ERR_UNAVAILABLE;
}

int32_t zenith_numa_set_membind(uint64_t nodemask) {
  (void)nodemask;
  return ZENITH_NUMA_ERR_UNAVAILABLE;
}

int32_t zenith_numa_get_node_info(int32_t node, ZenithNumaNodeInfo *info) {
  if (info == nullptr) {
    return ZENITH_NUMA_ERR_NULL_PTR;
  }
  if (node != 0) {
    return ZENITH_NUMA_ERR_INVALID_NODE;
  }
  info->node_id = 0;
  info->total_memory = 0;
  info->free_memory = 0;
  info->num_cpus = 1;
  return ZENITH_NUMA_OK;
}

int32_t zenith_numa_distance(int32_t node1, int32_t node2) {
  (void)node1;
  (void)node2;
  return 10; // Default local distance
}

} // extern "C"

#endif // ZENITH_USE_LIBNUMA
