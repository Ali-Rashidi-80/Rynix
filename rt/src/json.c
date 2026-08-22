#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Minimal JSON field extractor for v0.1 std surface.
 * Parses {"key": 123} or {"key":123} — returns the integer value or -1. */
int64_t rynix_rt_json_get_i64(const char *json, const char *key) {
  if (!json || !key) {
    return -1;
  }
  size_t klen = strlen(key);
  const char *p = json;
  while ((p = strstr(p, key)) != NULL) {
    /* Require quoted key */
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
            return (int64_t)v;
          }
        }
      }
    }
    p += klen;
  }
  return -1;
}
