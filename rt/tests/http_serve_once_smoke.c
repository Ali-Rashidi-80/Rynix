/* HTTP serve-once + client GET smoke (fibers). */

#include "../include/rynix_rt.h"

#include <stdio.h>

static int g_ok;
static int64_t g_got = -1;

static void server_fiber(void *arg) {
  (void)arg;
  int64_t rc = rynix_rt_http_serve_once_json_i64(40123, "/api", 42);
  if (rc != 0) {
    fprintf(stderr, "serve_once failed: %lld\n", (long long)rc);
  }
}

static void client_fiber(void *arg) {
  (void)arg;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    g_got = rynix_rt_http_get_json_i64("127.0.0.1", 40123, "/api", "value");
    if (g_got >= 0) {
      break;
    }
  }
  g_ok = (g_got == 42);
}

int main(void) {
  if (!rynix_rt_spawn(server_fiber, NULL)) {
    return 1;
  }
  if (!rynix_rt_spawn(client_fiber, NULL)) {
    return 1;
  }
  rynix_rt_run();
  if (!g_ok) {
    fprintf(stderr, "http_serve_once smoke failed got=%lld\n", (long long)g_got);
    return 1;
  }
  puts("http_serve_once ok");
  return 0;
}
