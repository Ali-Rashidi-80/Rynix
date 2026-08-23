/* Rynix runtime C ABI — Phase 7–9.
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
void rynix_rt_print_i64(int64_t n);
/* Optimizer barrier: returns x, but must not be constant-folded through. */
int64_t rynix_rt_opaque_i64(int64_t x);
void rynix_rt_panic(const char *msg);

/* ---- heap / regions ---------------------------------------------------- */
void *rynix_rt_heap_alloc(int64_t size);
void rynix_rt_heap_free(void *p);
void rynix_rt_region_create(int32_t id);
void rynix_rt_region_reset(int32_t id);
void *rynix_rt_region_alloc(int32_t id, int64_t size);
int64_t rynix_rt_now_ms(void);

/* ---- region Vec / Map (i64 monomorphized) ------------------------------ */
void *rynix_rt_vec_i64_new(int32_t region);
void rynix_rt_vec_i64_push(void *vec, int64_t x);
int64_t rynix_rt_vec_i64_get(void *vec, int64_t i);
int64_t rynix_rt_vec_i64_len(void *vec);
void *rynix_rt_map_i64_new(int32_t region);
void rynix_rt_map_i64_insert(void *map, int64_t key, int64_t val);
int64_t rynix_rt_map_i64_get(void *map, int64_t key);
int64_t rynix_rt_map_i64_len(void *map);

/* ---- fibers ------------------------------------------------------------ */
typedef void (*rynix_rt_fiber_fn)(void *arg);
void *rynix_rt_spawn(rynix_rt_fiber_fn fn, void *arg);
void rynix_rt_yield(void);
void rynix_rt_sleep_ms(int64_t ms);
void rynix_rt_run(void);
int64_t rynix_rt_fiber_count(void);

/* Fiber parking (used by fiber-aware io_uring). */
void *rynix_rt_fiber_current(void);
void rynix_rt_fiber_park(void);
void rynix_rt_fiber_unpark(void *fiber);
void rynix_rt_fiber_set_result(void *fiber, int64_t result);
int64_t rynix_rt_fiber_get_result(void *fiber);
int rynix_rt_fiber_parked_count(void);

/* ---- colorless I/O ----------------------------------------------------- */
int64_t rynix_rt_read(int64_t fd, void *buf, int64_t n);
int64_t rynix_rt_write(int64_t fd, const void *buf, int64_t n);

/* ---- TCP (portable / uring-aware) -------------------------------------- */
int64_t rynix_rt_tcp_listen(int64_t port);
int64_t rynix_rt_tcp_accept(int64_t listen_fd);
int64_t rynix_rt_tcp_connect(const char *host, int64_t port);
void rynix_rt_tcp_close(int64_t fd);
int64_t rynix_rt_tcp_recv(int64_t fd, void *buf, int64_t n);
int64_t rynix_rt_tcp_send(int64_t fd, const void *buf, int64_t n);

/* ---- JSON / HTTP (soft std) ------------------------------------------- */
int64_t rynix_rt_json_get_i64(const char *json, const char *key);
int64_t rynix_rt_http_get_json_i64(const char *host, int64_t port, const char *path,
                                   const char *field);

/* ---- io_uring (Linux + RYNIX_RT_URING; else stubs returning -1) -------- */
void rynix_rt_uring_init(void);
void rynix_rt_uring_shutdown(void);
int rynix_rt_uring_ready(void);
void rynix_rt_uring_poll(void);
void rynix_rt_uring_wait(void);
int64_t rynix_rt_uring_read(int64_t fd, void *buf, int64_t n);
int64_t rynix_rt_uring_write(int64_t fd, const void *buf, int64_t n);
int64_t rynix_rt_uring_accept(int64_t listen_fd);
int64_t rynix_rt_uring_connect(int64_t fd, const void *addr, int64_t addrlen);

#ifdef __cplusplus
}
#endif

#endif /* RYNIX_RT_H */
