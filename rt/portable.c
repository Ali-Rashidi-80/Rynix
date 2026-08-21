/* Portable Rynix runtime stubs (Phase 7 / Phase 8 portable backend).
 *
 * Linked by `rynixc build` via:
 *   clang -O3 -flto=thin -ffunction-sections … out.ll rt/portable.c -o out
 *
 * Symbol set is documented in docs/abi.md.
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ---- I/O --------------------------------------------------------------- */

void rynix_rt_print(const char *s) {
  if (s) {
    fputs(s, stdout);
    fputc('\n', stdout);
  }
}

void rynix_rt_panic(const char *msg) {
  fprintf(stderr, "rynix panic: %s\n", msg ? msg : "(null)");
  abort();
}

/* ---- Heap -------------------------------------------------------------- */

void *rynix_rt_heap_alloc(int64_t size) {
  if (size <= 0) size = 1;
  void *p = malloc((size_t)size);
  if (!p) rynix_rt_panic("out of memory");
  memset(p, 0, (size_t)size);
  return p;
}

void rynix_rt_heap_free(void *p) { free(p); }

/* ---- Regions (bump arenas) --------------------------------------------- */

#define RYNIX_MAX_REGIONS 16
#define RYNIX_REGION_CAP (1u << 20) /* 1 MiB per region in v0 */

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
  /* 8-byte align */
  need = (need + 7u) & ~((size_t)7u);
  if (r->len + need > r->cap) rynix_rt_panic("region overflow");
  void *p = r->base + r->len;
  r->len += need;
  memset(p, 0, need);
  return p;
}
