/* Ultra-thin Suite5 `--bench` runtime: sink print only (no stdio/getenv/heap). */

#include "rynix_rt.h"

void rynix_rt_print(const char *s) { (void)s; }

void rynix_rt_print_i64(int64_t n) {
  static volatile int64_t rynix_bench_sink;
  rynix_bench_sink = n;
}

int64_t rynix_rt_opaque_i64(int64_t x) {
  volatile int64_t v = x;
  return v;
}

void rynix_rt_panic(const char *msg) {
  (void)msg;
  for (;;) {
  }
}

void *rynix_rt_heap_alloc(int64_t size) {
  (void)size;
  return (void *)0;
}

void rynix_rt_heap_free(void *p) { (void)p; }

void rynix_rt_region_create(int32_t id) { (void)id; }

void rynix_rt_region_reset(int32_t id) { (void)id; }

void *rynix_rt_region_alloc(int32_t id, int64_t size) {
  (void)id;
  (void)size;
  return (void *)0;
}

int64_t rynix_rt_now_ms(void) { return 0; }
