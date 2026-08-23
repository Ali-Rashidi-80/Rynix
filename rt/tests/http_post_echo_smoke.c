/* HTTP POST + echo-json serve-once smoke (fibers). */

#include "../include/rynix_rt.h"

#include <stdio.h>

static int g_ok;
static int64_t g_got = -1;
static int64_t g_serve = -99;

static void server_fiber(void *arg) {
  (void)arg;
  g_serve = rynix_rt_http_serve_once_echo_json_i64(40126, "/echo", "n");
}

static void client_fiber(void *arg) {
  (void)arg;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    g_got = rynix_rt_http_post_json_i64("127.0.0.1", 40126, "/echo", "{\"n\": 99}",
                                        "value");
    if (g_got >= 0) {
      break;
    }
  }
  g_ok = (g_got == 99 && g_serve == 99);
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
    fprintf(stderr, "http_post_echo smoke failed got=%lld serve=%lld\n",
            (long long)g_got, (long long)g_serve);
    return 1;
  }
  puts("http_post_echo ok");
  return 0;
}
