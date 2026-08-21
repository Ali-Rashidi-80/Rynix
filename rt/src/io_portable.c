/* Colorless I/O — prefer io_uring when ready, else portable blocking. */

#include "rynix_rt.h"

#include <stdio.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <io.h>
#include <windows.h>
#else
#include <unistd.h>
#endif

int64_t rynix_rt_read(int64_t fd, void *buf, int64_t n) {
  if (!buf || n <= 0) return 0;
  int64_t ur = rynix_rt_uring_read(fd, buf, n);
  if (ur >= 0) return ur;
  rynix_rt_yield();
#ifdef _WIN32
  int r = _read((int)fd, buf, (unsigned)n);
  return r < 0 ? -1 : r;
#else
  ssize_t r = read((int)fd, buf, (size_t)n);
  return r < 0 ? -1 : (int64_t)r;
#endif
}

int64_t rynix_rt_write(int64_t fd, const void *buf, int64_t n) {
  if (!buf || n <= 0) return 0;
  int64_t ur = rynix_rt_uring_write(fd, buf, n);
  if (ur >= 0) return ur;
  rynix_rt_yield();
#ifdef _WIN32
  int r = _write((int)fd, buf, (unsigned)n);
  return r < 0 ? -1 : r;
#else
  ssize_t r = write((int)fd, buf, (size_t)n);
  return r < 0 ? -1 : (int64_t)r;
#endif
}
