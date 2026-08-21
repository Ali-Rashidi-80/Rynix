/* Rynix runtime C ABI — Phase 7/8.
 * See docs/abi.md for the full symbol table.
 */
#ifndef RYNIX_RT_H
#define RYNIX_RT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- panic / print ----------------------------------------------------- */
void rynix_rt_print(const char *s);
void rynix_rt_panic(const char *msg);

/* ---- heap / regions ---------------------------------------------------- */
void *rynix_rt_heap_alloc(int64_t size);
void rynix_rt_heap_free(void *p);
void rynix_rt_region_create(int32_t id);
void rynix_rt_region_reset(int32_t id);
void *rynix_rt_region_alloc(int32_t id, int64_t size);

/** Monotonic-ish milliseconds since an unspecified epoch (portable clock). */
int64_t rynix_rt_now_ms(void);

/* ---- fibers ------------------------------------------------------------ */
typedef void (*rynix_rt_fiber_fn)(void *arg);

/** Spawn a fiber on the current worker; returns an opaque handle (or NULL). */
void *rynix_rt_spawn(rynix_rt_fiber_fn fn, void *arg);

/** Cooperative yield to the next ready fiber on this worker. */
void rynix_rt_yield(void);

/** Sleep at least `ms` milliseconds (colorless; parks the fiber). */
void rynix_rt_sleep_ms(int64_t ms);

/** Run the scheduler until all fibers complete. Call from main thread. */
void rynix_rt_run(void);

/** Number of live (non-finished) fibers — used by leak checks. */
int64_t rynix_rt_fiber_count(void);

/* ---- colorless I/O (portable = blocking under the hood) ---------------- */
/** Read up to `n` bytes from fd into buf; returns bytes read or -1. */
int64_t rynix_rt_read(int64_t fd, void *buf, int64_t n);

/** Write `n` bytes; returns bytes written or -1. */
int64_t rynix_rt_write(int64_t fd, const void *buf, int64_t n);

#ifdef __cplusplus
}
#endif

#endif /* RYNIX_RT_H */
