/**
 * Zenith NUMA Backend - C FFI Header
 *
 * Native NUMA memory operations for high-performance ML data loading.
 * Provides NUMA-aware memory allocation, thread binding, and topology queries.
 *
 * Copyright 2025 Zenith Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

#ifndef ZENITH_NUMA_H
#define ZENITH_NUMA_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Error codes */
#define ZENITH_NUMA_OK 0
#define ZENITH_NUMA_ERR_UNAVAILABLE -1
#define ZENITH_NUMA_ERR_INVALID_NODE -2
#define ZENITH_NUMA_ERR_ALLOC_FAILED -3
#define ZENITH_NUMA_ERR_BIND_FAILED -4
#define ZENITH_NUMA_ERR_NULL_PTR -5

/* ============================================================================
 * Initialization and Cleanup
 * ============================================================================
 */

/**
 * Initialize the NUMA subsystem.
 * Must be called before any other NUMA functions.
 *
 * @return ZENITH_NUMA_OK on success, ZENITH_NUMA_ERR_UNAVAILABLE if NUMA not
 * supported
 */
int32_t zenith_numa_init(void);

/**
 * Cleanup NUMA subsystem resources.
 * Should be called when NUMA operations are no longer needed.
 */
void zenith_numa_cleanup(void);

/**
 * Check if NUMA is available on this system.
 *
 * @return 1 if available, 0 if not
 */
int32_t zenith_numa_available(void);

/* ============================================================================
 * Topology Queries
 * ============================================================================
 */

/**
 * Get the number of NUMA nodes in the system.
 *
 * @return Number of configured NUMA nodes
 */
int32_t zenith_numa_num_nodes(void);

/**
 * Get the total number of CPUs in the system.
 *
 * @return Number of configured CPUs
 */
int32_t zenith_numa_num_cpus(void);

/**
 * Get the NUMA node for a given CPU.
 *
 * @param cpu CPU ID
 * @return NUMA node ID, or -1 on error
 */
int32_t zenith_numa_node_of_cpu(int32_t cpu);

/**
 * Get the preferred NUMA node for the current thread.
 *
 * @return Preferred node ID
 */
int32_t zenith_numa_preferred_node(void);

/* ============================================================================
 * Memory Allocation
 * ============================================================================
 */

/**
 * Allocate memory on a specific NUMA node.
 *
 * @param size Size in bytes to allocate
 * @param node NUMA node ID
 * @return Pointer to allocated memory, or NULL on failure
 */
void *zenith_numa_alloc_onnode(size_t size, int32_t node);

/**
 * Allocate memory interleaved across all NUMA nodes.
 * Useful for data accessed by threads on different nodes.
 *
 * @param size Size in bytes to allocate
 * @return Pointer to allocated memory, or NULL on failure
 */
void *zenith_numa_alloc_interleaved(size_t size);

/**
 * Allocate memory on the local NUMA node.
 * Memory is allocated on the node closest to the calling thread.
 *
 * @param size Size in bytes to allocate
 * @return Pointer to allocated memory, or NULL on failure
 */
void *zenith_numa_alloc_local(size_t size);

/**
 * Free NUMA-allocated memory.
 *
 * @param ptr Pointer to memory allocated by zenith_numa_alloc_* functions
 * @param size Size of the allocation
 */
void zenith_numa_free(void *ptr, size_t size);

/* ============================================================================
 * Thread Binding
 * ============================================================================
 */

/**
 * Bind the current thread to run on CPUs of a specific NUMA node.
 *
 * @param node NUMA node ID
 * @return ZENITH_NUMA_OK on success, error code on failure
 */
int32_t zenith_numa_bind_thread_to_node(int32_t node);

/**
 * Bind the current thread to a specific CPU.
 *
 * @param cpu CPU ID
 * @return ZENITH_NUMA_OK on success, error code on failure
 */
int32_t zenith_numa_bind_thread_to_cpu(int32_t cpu);

/**
 * Unbind the current thread, allowing it to run on any CPU.
 *
 * @return ZENITH_NUMA_OK on success
 */
int32_t zenith_numa_unbind_thread(void);

/* ============================================================================
 * Memory Policies
 * ============================================================================
 */

/**
 * Set the preferred NUMA node for future memory allocations.
 *
 * @param node NUMA node ID (-1 for local allocation)
 * @return ZENITH_NUMA_OK on success
 */
int32_t zenith_numa_set_preferred(int32_t node);

/**
 * Set interleaved memory allocation across specified nodes.
 *
 * @param nodemask Bitmask of NUMA nodes (bit N = node N)
 * @return ZENITH_NUMA_OK on success
 */
int32_t zenith_numa_set_interleave(uint64_t nodemask);

/**
 * Bind memory allocations to specified nodes only.
 *
 * @param nodemask Bitmask of NUMA nodes (bit N = node N)
 * @return ZENITH_NUMA_OK on success
 */
int32_t zenith_numa_set_membind(uint64_t nodemask);

/* ============================================================================
 * Statistics and Information
 * ============================================================================
 */

/**
 * Information about a NUMA node.
 */
typedef struct {
  int32_t node_id;       /**< NUMA node ID */
  uint64_t total_memory; /**< Total memory in bytes */
  uint64_t free_memory;  /**< Free memory in bytes */
  int32_t num_cpus;      /**< Number of CPUs on this node */
} ZenithNumaNodeInfo;

/**
 * Get information about a specific NUMA node.
 *
 * @param node NUMA node ID
 * @param info Pointer to struct to fill with node information
 * @return ZENITH_NUMA_OK on success, error code on failure
 */
int32_t zenith_numa_get_node_info(int32_t node, ZenithNumaNodeInfo *info);

/**
 * Get the distance between two NUMA nodes.
 * Lower values indicate closer/faster access.
 *
 * @param node1 First NUMA node ID
 * @param node2 Second NUMA node ID
 * @return Distance value, or -1 on error
 */
int32_t zenith_numa_distance(int32_t node1, int32_t node2);

#ifdef __cplusplus
}
#endif

#endif /* ZENITH_NUMA_H */
