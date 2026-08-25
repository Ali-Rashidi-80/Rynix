/* Dual-path HTTP serve-loop: mix GETs on /a and /b until max_reqs. */

#include "../include/rynix_rt.h"

#include <stdio.h>

static int g_ok;
static int64_t g_serve = -99;
static int64_t g_got[3] = {-1, -1, -1};

static void server_fiber(void *arg) {
  (void)arg;
  if (rynix_rt_http_serve_loop_2paths_json_i64(40128, "/a", 11, "/b", 22, 0) != -1) {
    fprintf(stderr, "expected max_reqs<=0 → -1\n");
    return;
  }
  g_serve = rynix_rt_http_serve_loop_2paths_json_i64(40128, "/a", 11, "/b", 22, 3);
}

static void client_fiber(void *arg) {
  (void)arg;
  const char *paths[3] = {"/a", "/b", "/a"};
  const int64_t want[3] = {11, 22, 11};
  for (int i = 0; i < 3; i++) {
    int64_t got = -1;
    for (int attempt = 0; attempt < 256; attempt++) {
      rynix_rt_yield();
      got = rynix_rt_http_get_json_i64("127.0.0.1", 40128, paths[i], "value");
      if (got >= 0) {
        break;
      }
    }
    g_got[i] = got;
  }
  g_ok = (g_serve == 0 && g_got[0] == want[0] && g_got[1] == want[1] && g_got[2] == want[2]);
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
    fprintf(stderr,
            "http_loop_2paths failed serve=%lld got=%lld,%lld,%lld\n",
            (long long)g_serve, (long long)g_got[0], (long long)g_got[1],
            (long long)g_got[2]);
    return 1;
  }
  puts("http_loop_2paths ok");
  return 0;
}
