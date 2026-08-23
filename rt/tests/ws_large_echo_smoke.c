/* Large WS frame echo on wire (16-bit extended length, ~70 KiB). */

#include "../include/rynix_rt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define WS_LARGE_N 70000
#define WS_LARGE_PORT 40132

static int g_ok;
static int64_t g_client = -99;
static int64_t g_serve = -99;
static char *g_msg;

static void server_fiber(void *arg) {
  (void)arg;
  g_serve = rynix_rt_ws_serve_once_echo_n(WS_LARGE_PORT, g_msg, WS_LARGE_N);
}

static void client_fiber(void *arg) {
  (void)arg;
  for (int attempt = 0; attempt < 512; attempt++) {
    rynix_rt_yield();
    g_client = rynix_rt_ws_client_echo_n("127.0.0.1", WS_LARGE_PORT, g_msg, WS_LARGE_N);
    if (g_client == 0) {
      break;
    }
  }
  g_ok = (g_client == 0 && g_serve == 0);
}

int main(void) {
  int i;
  g_msg = (char *)malloc(WS_LARGE_N);
  if (!g_msg) {
    return 1;
  }
  for (i = 0; i < WS_LARGE_N; i++) {
    g_msg[i] = (char)('A' + (i % 26));
  }
  if (!rynix_rt_spawn(server_fiber, NULL) || !rynix_rt_spawn(client_fiber, NULL)) {
    free(g_msg);
    return 1;
  }
  rynix_rt_run();
  free(g_msg);
  if (!g_ok) {
    fprintf(stderr, "ws large echo failed client=%lld serve=%lld\n", (long long)g_client,
            (long long)g_serve);
    return 1;
  }
  puts("ws_large_echo ok");
  return 0;
}
