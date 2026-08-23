/* Heap + bump-region allocators (shared by all backends). */

#include "rynix_rt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <time.h>
#endif

void rynix_rt_print(const char *s) {
  if (s) {
    fputs(s, stdout);
    fputc('\n', stdout);
    fflush(stdout);
  }
}

int64_t rynix_rt_opaque_i64(int64_t x) {
  volatile int64_t v = x;
  return v;
}

void rynix_rt_print_i64(int64_t n) {
#ifdef RYNIX_BENCH
  /* Suite5 `--bench` binaries always sink: no getenv/printf on the timed path.
   * Checksum verification uses a separate non-`--bench` build (see run_suite5.py). */
  static volatile int64_t rynix_bench_sink;
  rynix_bench_sink = n;
  return;
#else
  if (getenv("SUITE5_BENCH")) {
    static volatile int64_t suite5_sink;
    suite5_sink = n;
    return;
  }
  printf("%lld\n", (long long)n);
  fflush(stdout);
#endif
}

void rynix_rt_panic(const char *msg) {
  fprintf(stderr, "rynix panic: %s\n", msg ? msg : "(null)");
  fflush(stderr);
  abort();
}

void *rynix_rt_heap_alloc(int64_t size) {
  if (size <= 0) size = 1;
  void *p = malloc((size_t)size);
  if (!p) rynix_rt_panic("out of memory");
  memset(p, 0, (size_t)size);
  return p;
}

void rynix_rt_heap_free(void *p) { free(p); }

int64_t rynix_rt_now_ms(void) {
#if defined(_WIN32)
  /* GetTickCount64 is ms since boot — good enough for portable clocks. */
  return (int64_t)GetTickCount64();
#else
  struct timespec ts;
  if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return 0;
  return (int64_t)ts.tv_sec * 1000 + (int64_t)ts.tv_nsec / 1000000;
#endif
}

#define RYNIX_MAX_REGIONS 16
#define RYNIX_REGION_CAP (1u << 20)

typedef struct {
  char *base;
  size_t len;
  size_t cap;
  int live;
} Region;

static Region g_regions[RYNIX_MAX_REGIONS];

void rynix_rt_region_create(int32_t id) {
  if (id < 0 || id >= RYNIX_MAX_REGIONS) rynix_rt_panic("bad region id");
  Region *r = &g_regions[id];
  if (!r->live) {
    r->base = (char *)malloc(RYNIX_REGION_CAP);
    if (!r->base) rynix_rt_panic("region alloc failed");
    r->cap = RYNIX_REGION_CAP;
    r->live = 1;
  }
  r->len = 0;
}

void rynix_rt_region_reset(int32_t id) {
  if (id < 0 || id >= RYNIX_MAX_REGIONS) return;
  Region *r = &g_regions[id];
  if (r->live) r->len = 0;
}

void *rynix_rt_region_alloc(int32_t id, int64_t size) {
  if (id < 0 || id >= RYNIX_MAX_REGIONS) rynix_rt_panic("bad region id");
  Region *r = &g_regions[id];
  if (!r->live) rynix_rt_region_create(id);
  size_t need = (size_t)(size < 0 ? 0 : size);
  need = (need + 7u) & ~((size_t)7u);
  if (r->len + need > r->cap) rynix_rt_panic("region overflow");
  void *p = r->base + r->len;
  r->len += need;
  memset(p, 0, need);
  return p;
}
