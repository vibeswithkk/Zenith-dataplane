#ifndef ZENITH_CORE_H
#define ZENITH_CORE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque engine handle
typedef void* ZenithEngine;

// Error codes
#define ZENITH_OK 0
#define ZENITH_ERR_NULL_PTR -1
#define ZENITH_ERR_BUFFER_FULL -2
#define ZENITH_ERR_PLUGIN_LOAD -3
#define ZENITH_ERR_FFI -4

// Engine lifecycle
ZenithEngine zenith_init(uint32_t buffer_size);
void zenith_free(ZenithEngine engine);

// Event publishing
int32_t zenith_publish(
    ZenithEngine engine,
    void* array_ptr,
    void* schema_ptr,
    uint32_t source_id,
    uint64_t seq_no
);

// Plugin management
int32_t zenith_load_plugin(
    ZenithEngine engine,
    const uint8_t* wasm_bytes,
    size_t len
);

// Engine statistics
typedef struct {
    size_t buffer_len;
    size_t plugin_count;
    uint64_t events_processed;
} ZenithStats;

int32_t zenith_get_stats(ZenithEngine engine, ZenithStats* stats);

// Admin API status
typedef struct {
    const char* status;
    size_t buffer_len;
    size_t plugin_count;
} ZenithStatus;

int32_t zenith_get_status(ZenithEngine engine, ZenithStatus* status);

#ifdef __cplusplus
}
#endif

#endif // ZENITH_CORE_H
