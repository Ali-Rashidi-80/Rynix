/* Region-backed Vec / Map (i64 elements / i64→i64) — monomorphized std surface. */

#include "rynix_rt.h"

#include <string.h>

typedef struct {
  int32_t region;
  int64_t *data;
  int64_t len;
  int64_t cap;
} rynix_vec_i64;

typedef struct {
  int64_t key;
  int64_t val;
  int used;
} rynix_map_slot;

typedef struct {
  int32_t region;
  rynix_map_slot *slots;
  int64_t cap;
  int64_t len;
} rynix_map_i64;

static void *reg_alloc(int32_t region, size_t n) {
  if (region >= 0) return rynix_rt_region_alloc(region, (int64_t)n);
  return rynix_rt_heap_alloc((int64_t)n);
}

void *rynix_rt_vec_i64_new(int32_t region) {
  rynix_vec_i64 *v = (rynix_vec_i64 *)reg_alloc(region, sizeof(rynix_vec_i64));
  if (!v) return NULL;
  v->region = region;
  v->data = NULL;
  v->len = 0;
  v->cap = 0;
  return v;
}

void rynix_rt_vec_i64_push(void *vec, int64_t x) {
  rynix_vec_i64 *v = (rynix_vec_i64 *)vec;
  if (!v) return;
  if (v->len >= v->cap) {
    int64_t ncap = v->cap == 0 ? 8 : v->cap * 2;
    int64_t *nd = (int64_t *)reg_alloc(v->region, (size_t)ncap * sizeof(int64_t));
    if (!nd) rynix_rt_panic("vec grow failed");
    if (v->data && v->len > 0) memcpy(nd, v->data, (size_t)v->len * sizeof(int64_t));
    v->data = nd;
    v->cap = ncap;
  }
  v->data[v->len++] = x;
}

int64_t rynix_rt_vec_i64_get(void *vec, int64_t i) {
  rynix_vec_i64 *v = (rynix_vec_i64 *)vec;
  if (!v || i < 0 || i >= v->len) rynix_rt_panic("vec index");
  return v->data[i];
}

int64_t rynix_rt_vec_i64_len(void *vec) {
  rynix_vec_i64 *v = (rynix_vec_i64 *)vec;
  return v ? v->len : 0;
}

void *rynix_rt_map_i64_new(int32_t region) {
  rynix_map_i64 *m = (rynix_map_i64 *)reg_alloc(region, sizeof(rynix_map_i64));
  if (!m) return NULL;
  m->region = region;
  m->cap = 16;
  m->len = 0;
  m->slots = (rynix_map_slot *)reg_alloc(region, (size_t)m->cap * sizeof(rynix_map_slot));
  if (!m->slots) rynix_rt_panic("map alloc");
  memset(m->slots, 0, (size_t)m->cap * sizeof(rynix_map_slot));
  return m;
}

static int64_t map_probe(rynix_map_i64 *m, int64_t key) {
  int64_t i = (key < 0 ? -key : key) % m->cap;
  for (int64_t n = 0; n < m->cap; n++) {
    int64_t j = (i + n) % m->cap;
    if (!m->slots[j].used || m->slots[j].key == key) return j;
  }
  return -1;
}

void rynix_rt_map_i64_insert(void *map, int64_t key, int64_t val) {
  rynix_map_i64 *m = (rynix_map_i64 *)map;
  if (!m) return;
  int64_t j = map_probe(m, key);
  if (j < 0) rynix_rt_panic("map full");
  if (!m->slots[j].used) {
    m->slots[j].used = 1;
    m->slots[j].key = key;
    m->len++;
  }
  m->slots[j].val = val;
}

int64_t rynix_rt_map_i64_get(void *map, int64_t key) {
  rynix_map_i64 *m = (rynix_map_i64 *)map;
  if (!m) return 0;
  int64_t j = map_probe(m, key);
  if (j < 0 || !m->slots[j].used) return 0;
  return m->slots[j].val;
}

int64_t rynix_rt_map_i64_len(void *map) {
  rynix_map_i64 *m = (rynix_map_i64 *)map;
  return m ? m->len : 0;
}
