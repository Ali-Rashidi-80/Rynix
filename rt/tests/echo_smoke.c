/* Fiber echo smoke — portable read/write round-trip (Phase 8/M8 honesty).
 *
 * Spawns a fiber that reads from a pipe/socketpair and writes back the same
 * bytes. Validates colorless I/O + scheduler without requiring io_uring.
 */

#include "../include/rynix_rt.h"

#include <stdio.h>
#include <string.h>

#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <io.h>
#include <fcntl.h>
#else
#include <unistd.h>
#endif

static int g_rdfd = -1;
static int g_wrfd = -1;
static char g_out[64];
static int64_t g_nout;

static void echo_fiber(void *arg) {
  (void)arg;
  char buf[64];
  int64_t n = rynix_rt_read((int64_t)g_rdfd, buf, (int64_t)sizeof(buf));
  if (n > 0) {
    g_nout = rynix_rt_write((int64_t)g_wrfd, buf, n);
    if (g_nout > 0 && g_nout < (int64_t)sizeof(g_out)) {
      memcpy(g_out, buf, (size_t)g_nout);
      g_out[g_nout] = 0;
    }
  }
}

int main(void) {
#if defined(_WIN32)
  int fds[2];
  if (_pipe(fds, 256, _O_BINARY) != 0) {
    fprintf(stderr, "pipe failed\n");
    return 1;
  }
  g_wrfd = fds[1];
  g_rdfd = fds[0];
#else
  int fds[2];
  if (pipe(fds) != 0) {
    perror("pipe");
    return 1;
  }
  g_rdfd = fds[0];
  g_wrfd = fds[1];
#endif

  const char *msg = "echo-ok";
  if (rynix_rt_write((int64_t)g_wrfd, msg, (int64_t)strlen(msg)) < 0) {
    fprintf(stderr, "write priming failed\n");
    return 1;
  }

  if (!rynix_rt_spawn(echo_fiber, NULL)) {
    fprintf(stderr, "spawn failed\n");
    return 1;
  }
  rynix_rt_run();

  if (strcmp(g_out, msg) != 0) {
    fprintf(stderr, "echo mismatch: got '%s'\n", g_out);
    return 1;
  }
  if (rynix_rt_fiber_count() != 0) {
    fprintf(stderr, "fiber leak: %lld\n", (long long)rynix_rt_fiber_count());
    return 1;
  }
  puts("echo_smoke ok");
  return 0;
}
