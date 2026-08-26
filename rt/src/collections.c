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

typedef struct {
  int32_t region;
  const char **data;
  int64_t len;
  int64_t cap;
} rynix_vec_str;

void *rynix_rt_vec_str_new(int32_t region) {
  rynix_vec_str *v = (rynix_vec_str *)reg_alloc(region, sizeof(rynix_vec_str));
  if (!v) return NULL;
  v->region = region;
  v->data = NULL;
  v->len = 0;
  v->cap = 0;
  return v;
}

void rynix_rt_vec_str_push(void *vec, const char *s) {
  rynix_vec_str *v = (rynix_vec_str *)vec;
  if (!v) return;
  if (v->len >= v->cap) {
    int64_t ncap = v->cap == 0 ? 8 : v->cap * 2;
    const char **nd =
        (const char **)reg_alloc(v->region, (size_t)ncap * sizeof(const char *));
    if (!nd) rynix_rt_panic("vec_str grow failed");
    if (v->data && v->len > 0)
      memcpy(nd, v->data, (size_t)v->len * sizeof(const char *));
    v->data = nd;
    v->cap = ncap;
  }
  v->data[v->len++] = s ? s : "";
}

const char *rynix_rt_vec_str_get(void *vec, int64_t i) {
  rynix_vec_str *v = (rynix_vec_str *)vec;
  if (!v || i < 0 || i >= v->len) rynix_rt_panic("vec_str index");
  return v->data[i];
}

int64_t rynix_rt_vec_str_len(void *vec) {
  rynix_vec_str *v = (rynix_vec_str *)vec;
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

static void map_grow(rynix_map_i64 *m) {
  int64_t old_cap = m->cap;
  rynix_map_slot *old = m->slots;
  int64_t ncap = old_cap * 2;
  if (ncap < 16) ncap = 16;
  rynix_map_slot *ns = (rynix_map_slot *)reg_alloc(m->region, (size_t)ncap * sizeof(rynix_map_slot));
  if (!ns) rynix_rt_panic("map grow");
  memset(ns, 0, (size_t)ncap * sizeof(rynix_map_slot));
  m->slots = ns;
  m->cap = ncap;
  m->len = 0;
  for (int64_t i = 0; i < old_cap; i++) {
    if (!old[i].used) continue;
    int64_t j = map_probe(m, old[i].key);
    if (j < 0) rynix_rt_panic("map grow full");
    m->slots[j].used = 1;
    m->slots[j].key = old[i].key;
    m->slots[j].val = old[i].val;
    m->len++;
  }
}

void rynix_rt_map_i64_insert(void *map, int64_t key, int64_t val) {
  rynix_map_i64 *m = (rynix_map_i64 *)map;
  if (!m) return;
  if (m->len * 2 >= m->cap) {
    map_grow(m);
  }
  int64_t j = map_probe(m, key);
  if (j < 0) {
    map_grow(m);
    j = map_probe(m, key);
  }
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

typedef struct {
  const char *key;
  int64_t val;
  int used;
} rynix_map_str_slot;

typedef struct {
  int32_t region;
  rynix_map_str_slot *slots;
  int64_t cap;
  int64_t len;
} rynix_map_str_i64;

static uint64_t str_hash(const char *s) {
  uint64_t h = 1469598103934665603ull;
  if (!s) return h;
  for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
    h ^= (uint64_t)(*p);
    h *= 1099511628211ull;
  }
  return h;
}

static int str_eq(const char *a, const char *b) {
  if (a == b) return 1;
  if (!a || !b) return 0;
  return strcmp(a, b) == 0;
}

static int64_t map_str_probe(rynix_map_str_i64 *m, const char *key) {
  int64_t i = (int64_t)(str_hash(key) % (uint64_t)m->cap);
  for (int64_t n = 0; n < m->cap; n++) {
    int64_t j = (i + n) % m->cap;
    if (!m->slots[j].used || str_eq(m->slots[j].key, key)) return j;
  }
  return -1;
}

static void map_str_grow(rynix_map_str_i64 *m) {
  int64_t old_cap = m->cap;
  rynix_map_str_slot *old = m->slots;
  int64_t ncap = old_cap * 2;
  if (ncap < 16) ncap = 16;
  rynix_map_str_slot *ns =
      (rynix_map_str_slot *)reg_alloc(m->region, (size_t)ncap * sizeof(rynix_map_str_slot));
  if (!ns) rynix_rt_panic("map_str grow");
  memset(ns, 0, (size_t)ncap * sizeof(rynix_map_str_slot));
  m->slots = ns;
  m->cap = ncap;
  m->len = 0;
  for (int64_t i = 0; i < old_cap; i++) {
    if (!old[i].used) continue;
    int64_t j = map_str_probe(m, old[i].key);
    if (j < 0) rynix_rt_panic("map_str grow full");
    m->slots[j].used = 1;
    m->slots[j].key = old[i].key;
    m->slots[j].val = old[i].val;
    m->len++;
  }
}

void *rynix_rt_map_str_i64_new(int32_t region) {
  rynix_map_str_i64 *m =
      (rynix_map_str_i64 *)reg_alloc(region, sizeof(rynix_map_str_i64));
  if (!m) return NULL;
  m->region = region;
  m->cap = 16;
  m->len = 0;
  m->slots =
      (rynix_map_str_slot *)reg_alloc(region, (size_t)m->cap * sizeof(rynix_map_str_slot));
  if (!m->slots) rynix_rt_panic("map_str alloc");
  memset(m->slots, 0, (size_t)m->cap * sizeof(rynix_map_str_slot));
  return m;
}

void rynix_rt_map_str_i64_insert(void *map, const char *key, int64_t val) {
  rynix_map_str_i64 *m = (rynix_map_str_i64 *)map;
  if (!m) return;
  if (!key) key = "";
  if (m->len * 2 >= m->cap) {
    map_str_grow(m);
  }
  int64_t j = map_str_probe(m, key);
  if (j < 0) {
    map_str_grow(m);
    j = map_str_probe(m, key);
  }
  if (j < 0) rynix_rt_panic("map_str full");
  if (!m->slots[j].used) {
    m->slots[j].used = 1;
    m->slots[j].key = key;
    m->len++;
  }
  m->slots[j].val = val;
}

int64_t rynix_rt_map_str_i64_get(void *map, const char *key) {
  rynix_map_str_i64 *m = (rynix_map_str_i64 *)map;
  if (!m) return 0;
  if (!key) key = "";
  int64_t j = map_str_probe(m, key);
  if (j < 0 || !m->slots[j].used) return 0;
  return m->slots[j].val;
}

int64_t rynix_rt_map_str_i64_len(void *map) {
  rynix_map_str_i64 *m = (rynix_map_str_i64 *)map;
  return m ? m->len : 0;
}

typedef struct {
  const char *key;
  const char *val;
  int used;
} rynix_map_ss_slot;

typedef struct {
  int32_t region;
  rynix_map_ss_slot *slots;
  int64_t cap;
  int64_t len;
} rynix_map_str_str;

static int64_t map_ss_probe(rynix_map_str_str *m, const char *key) {
  int64_t i = (int64_t)(str_hash(key) % (uint64_t)m->cap);
  for (int64_t n = 0; n < m->cap; n++) {
    int64_t j = (i + n) % m->cap;
    if (!m->slots[j].used || str_eq(m->slots[j].key, key)) return j;
  }
  return -1;
}

static void map_ss_grow(rynix_map_str_str *m) {
  int64_t old_cap = m->cap;
  rynix_map_ss_slot *old = m->slots;
  int64_t ncap = old_cap * 2;
  if (ncap < 16) ncap = 16;
  rynix_map_ss_slot *ns =
      (rynix_map_ss_slot *)reg_alloc(m->region, (size_t)ncap * sizeof(rynix_map_ss_slot));
  if (!ns) rynix_rt_panic("map_ss grow");
  memset(ns, 0, (size_t)ncap * sizeof(rynix_map_ss_slot));
  m->slots = ns;
  m->cap = ncap;
  m->len = 0;
  for (int64_t i = 0; i < old_cap; i++) {
    if (!old[i].used) continue;
    int64_t j = map_ss_probe(m, old[i].key);
    if (j < 0) rynix_rt_panic("map_ss grow full");
    m->slots[j].used = 1;
    m->slots[j].key = old[i].key;
    m->slots[j].val = old[i].val;
    m->len++;
  }
}

void *rynix_rt_map_str_str_new(int32_t region) {
  rynix_map_str_str *m =
      (rynix_map_str_str *)reg_alloc(region, sizeof(rynix_map_str_str));
  if (!m) return NULL;
  m->region = region;
  m->cap = 16;
  m->len = 0;
  m->slots =
      (rynix_map_ss_slot *)reg_alloc(region, (size_t)m->cap * sizeof(rynix_map_ss_slot));
  if (!m->slots) rynix_rt_panic("map_ss alloc");
  memset(m->slots, 0, (size_t)m->cap * sizeof(rynix_map_ss_slot));
  return m;
}

void rynix_rt_map_str_str_insert(void *map, const char *key, const char *val) {
  rynix_map_str_str *m = (rynix_map_str_str *)map;
  if (!m) return;
  if (!key) key = "";
  if (!val) val = "";
  if (m->len * 2 >= m->cap) {
    map_ss_grow(m);
  }
  int64_t j = map_ss_probe(m, key);
  if (j < 0) {
    map_ss_grow(m);
    j = map_ss_probe(m, key);
  }
  if (j < 0) rynix_rt_panic("map_ss full");
  if (!m->slots[j].used) {
    m->slots[j].used = 1;
    m->slots[j].key = key;
    m->len++;
  }
  m->slots[j].val = val;
}

const char *rynix_rt_map_str_str_get(void *map, const char *key) {
  rynix_map_str_str *m = (rynix_map_str_str *)map;
  if (!m) return "";
  if (!key) key = "";
  int64_t j = map_ss_probe(m, key);
  if (j < 0 || !m->slots[j].used) return "";
  return m->slots[j].val ? m->slots[j].val : "";
}

int64_t rynix_rt_map_str_str_len(void *map) {
  rynix_map_str_str *m = (rynix_map_str_str *)map;
  return m ? m->len : 0;
}
