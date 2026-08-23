/* Portable whole-file read/write for soft std `fs_*`. */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../include/rynix_rt.h"

/* 0 on success, -1 on failure. */
int64_t rynix_rt_fs_write_file(const char *path, const char *data) {
  FILE *f;
  size_t n;
  size_t w;
  if (!path || !data) {
    return -1;
  }
  f = fopen(path, "wb");
  if (!f) {
    return -1;
  }
  n = strlen(data);
  w = fwrite(data, 1, n, f);
  if (fclose(f) != 0 || w != n) {
    return -1;
  }
  return 0;
}

/* Heap NUL-terminated contents, or NULL on failure. Caller may free. */
char *rynix_rt_fs_read_file(const char *path) {
  FILE *f;
  long sz;
  char *buf;
  size_t got;
  if (!path) {
    return NULL;
  }
  f = fopen(path, "rb");
  if (!f) {
    return NULL;
  }
  if (fseek(f, 0, SEEK_END) != 0) {
    fclose(f);
    return NULL;
  }
  sz = ftell(f);
  if (sz < 0) {
    fclose(f);
    return NULL;
  }
  if (fseek(f, 0, SEEK_SET) != 0) {
    fclose(f);
    return NULL;
  }
  buf = (char *)malloc((size_t)sz + 1);
  if (!buf) {
    fclose(f);
    return NULL;
  }
  got = fread(buf, 1, (size_t)sz, f);
  fclose(f);
  if (got != (size_t)sz) {
    free(buf);
    return NULL;
  }
  buf[sz] = '\0';
  return buf;
}

/* 0 if file contents equal expect, else -1. */
int64_t rynix_rt_fs_read_file_eq(const char *path, const char *expect) {
  char *got;
  int64_t ok;
  if (!expect) {
    return -1;
  }
  got = rynix_rt_fs_read_file(path);
  if (!got) {
    return -1;
  }
  ok = (strcmp(got, expect) == 0) ? 0 : -1;
  free(got);
  return ok;
}

/* 1 if path exists as a regular file (or any fopen-readable), else 0. */
int64_t rynix_rt_fs_exists(const char *path) {
  FILE *f;
  if (!path) {
    return 0;
  }
  f = fopen(path, "rb");
  if (!f) {
    return 0;
  }
  fclose(f);
  return 1;
}

/* 0 on success, -1 on failure. Missing path is success (idempotent). */
int64_t rynix_rt_fs_remove_file(const char *path) {
  FILE *probe;
  if (!path) {
    return -1;
  }
  if (remove(path) == 0) {
    return 0;
  }
  /* Already gone → ok. */
  probe = fopen(path, "rb");
  if (!probe) {
    return 0;
  }
  fclose(probe);
  return -1;
}
