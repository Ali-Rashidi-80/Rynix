#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Minimal JSON field helpers for soft std.
 * Parses {"key": 123} / {"key":123} — integer object fields only. */

static const char *rynix_json_find_int(const char *json, const char *key, long long *out) {
  if (!json || !key || !out) {
    return NULL;
  }
  size_t klen = strlen(key);
  const char *p = json;
  while ((p = strstr(p, key)) != NULL) {
    if (p == json || p[-1] == '"') {
      const char *after = p + klen;
      if (*after == '"') {
        after++;
        while (*after == ' ' || *after == '\t' || *after == '\n') {
          after++;
        }
        if (*after == ':') {
          after++;
          while (*after == ' ' || *after == '\t' || *after == '\n') {
            after++;
          }
          char *end = NULL;
          long long v = strtoll(after, &end, 10);
          if (end != after) {
            *out = v;
            return after;
          }
        }
      }
    }
    p += klen;
  }
  return NULL;
}

int64_t rynix_rt_json_get_i64(const char *json, const char *key) {
  long long v = 0;
  if (!rynix_json_find_int(json, key, &v)) {
    return -1;
  }
  return (int64_t)v;
}

/* 1 if `key` maps to an integer field, else 0. */
int64_t rynix_rt_json_has_i64(const char *json, const char *key) {
  long long v = 0;
  return rynix_json_find_int(json, key, &v) ? 1 : 0;
}
