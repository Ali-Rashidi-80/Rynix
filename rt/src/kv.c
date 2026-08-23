/* String-key → i64 arena KV (soft EndKV-class surface). */

#include <stdint.h>
#include <string.h>

#include "../include/rynix_rt.h"

typedef struct KvEntry {
  const char *key;
  int64_t value;
  struct KvEntry *next;
} KvEntry;

typedef struct {
  int32_t region;
  KvEntry *head;
  int64_t len;
} KvStore;

void *rynix_rt_kv_new(int32_t region) {
  KvStore *kv = (KvStore *)rynix_rt_region_alloc(region, (int64_t)sizeof(KvStore));
  if (!kv) {
    return NULL;
  }
  kv->region = region;
  kv->head = NULL;
  kv->len = 0;
  return kv;
}

void rynix_rt_kv_put(void *kv_ptr, const char *key, int64_t value) {
  KvStore *kv = (KvStore *)kv_ptr;
  KvEntry *e;
  size_t klen;
  char *copy;
  if (!kv || !key) {
    return;
  }
  for (e = kv->head; e; e = e->next) {
    if (strcmp(e->key, key) == 0) {
      e->value = value;
      return;
    }
  }
  klen = strlen(key);
  e = (KvEntry *)rynix_rt_region_alloc(kv->region, (int64_t)sizeof(KvEntry));
  copy = (char *)rynix_rt_region_alloc(kv->region, (int64_t)klen + 1);
  if (!e || !copy) {
    return;
  }
  memcpy(copy, key, klen + 1);
  e->key = copy;
  e->value = value;
  e->next = kv->head;
  kv->head = e;
  kv->len += 1;
}

int64_t rynix_rt_kv_get(void *kv_ptr, const char *key) {
  KvStore *kv = (KvStore *)kv_ptr;
  KvEntry *e;
  if (!kv || !key) {
    return 0;
  }
  for (e = kv->head; e; e = e->next) {
    if (strcmp(e->key, key) == 0) {
      return e->value;
    }
  }
  return 0;
}

int64_t rynix_rt_kv_len(void *kv_ptr) {
  KvStore *kv = (KvStore *)kv_ptr;
  return kv ? kv->len : 0;
}
